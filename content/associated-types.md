# Associated Types

In the first part of this book, we have learned about how CGP makes use of
Rust's trait system to wire up components using blanket implementations.
Since CGP works within Rust's trait system, we can make use of advanced
Rust features together with CGP to form new design patterns.
In this chapter, we will learn about how to make use of _associated types_
with CGP to define context-generic providers that are generic over multiple
_abstract_ types.

## Building Authentication Components

Supposed that we want to build a simple authentication system using _bearer tokens_
with expiry time. To build such system, we would need to fetch the expiry time of
a valid token, and ensure that the time is not in the past. A naive attempt of
implementing the authentication would be as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# pub mod main {
pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    #[cgp_component {
        provider: AuthTokenValidator,
    }]
    pub trait CanValidateAuthToken {
        fn validate_auth_token(&self, auth_token: &str) -> Result<(), Error>;
    }

    #[cgp_component {
        provider: AuthTokenExpiryFetcher,
    }]
    pub trait CanFetchAuthTokenExpiry {
        fn fetch_auth_token_expiry(&self, auth_token: &str) -> Result<u64, Error>;
    }

    #[cgp_component {
        provider: CurrentTimeGetter,
    }]
    pub trait HasCurrentTime {
        fn current_time(&self) -> Result<u64, Error>;
    }
}

pub mod impls {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::{anyhow, Error};

    use super::traits::*;

    pub struct ValidateTokenIsNotExpired;

    impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
    where
        Context: HasCurrentTime + CanFetchAuthTokenExpiry,
    {
        fn validate_auth_token(context: &Context, auth_token: &str) -> Result<(), Error> {
            let now = context.current_time()?;

            let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

            if token_expiry < now {
                Ok(())
            } else {
                Err(anyhow!("auth token has expired"))
            }
        }
    }

    pub struct GetSystemTimestamp;

    impl<Context> CurrentTimeGetter<Context> for GetSystemTimestamp {
        fn current_time(_context: &Context) -> Result<u64, Error> {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis()
                .try_into()?;

            Ok(now)
        }
    }
}

pub mod contexts {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use cgp::prelude::*;

    use super::impls::*;
    use super::traits::*;

    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, u64>,
    }

    pub struct MockAppComponents;

    impl HasComponents for MockApp {
        type Components = MockAppComponents;
    }

    delegate_components! {
        MockAppComponents {
            CurrentTimeGetterComponent: GetSystemTimestamp,
            AuthTokenValidatorComponent: ValidateTokenIsNotExpired,
        }
    }

    impl AuthTokenExpiryFetcher<MockApp> for MockAppComponents {
        fn fetch_auth_token_expiry(
            context: &MockApp,
            auth_token: &str,
        ) -> Result<u64, anyhow::Error> {
            context
                .auth_tokens_store
                .get(auth_token)
                .cloned()
                .ok_or_else(|| anyhow!("invalid auth token"))
        }
    }

    pub trait CanUseMockApp: CanValidateAuthToken {}

    impl CanUseMockApp for MockApp {}
}
#
# }
```

We first define `CanValidateAuthToken`, which would be used as the main API for validating an
auth token. In order to help implementing the validator, we also define
`CanFetchAuthTokenExpiry` used for fetching the expiry time of an auth token, if the token is valid.
Finally, we also define `HasCurrentTime` which is used for fetching the current time.

We then define a context-generic provider `ValidateTokenIsNotExpired`, which validates auth tokens
by fetching the token's expiry time and the current time, and ensure that the token's expiry time
does not exceed the current time. We also define a context-generic provider `GetSystemTimestamp`,
which gets the current time using `std::time::System::now()`.

For the purpose of this demo, we also define a concrete context `MockApp`, which contains a
`auth_tokens_store` field with mocked collection of auth tokens with respective expiry time
stored in a `BTreeMap`.
We then implement a context-specific provider of `AuthTokenExpiryFetcher` for `MockApp`,
which reads from the mocked `auth_tokens_store`.
We also define a check trait `CanUseMockApp`, to check that `MockApp` correctly implements
`CanValidateAuthToken` with the wiring provided.

## Abstract Types

The naive example above makes use of basic CGP techniques to implement a reusable provider
`ValidateTokenIsNotExpired`, which can be used with different concrete contexts.
However, we can see that the method signatures are tied to specific types.
In particular, we used `String` to represent the auth token, and `u64` to
represent the unix timestamp in milliseconds.

Common wisdom tells us that we should use distinct types to distinguish values
from specific domains, so that we do not accidentally mix up values from different
domains. A common approach in Rust is to make use of the _newtype pattern_ to
define wrapper types such as follows:

```rust
pub struct AuthToken {
    value: String,
}

pub struct Time {
    value: u64,
}
```

Although the newtype pattern abstracts over the underlying value, it does not allow
our code to be generalized over distinct types. For example, instead of defining
our own `Time` type with Unix timestamp semantics, we may want to use a datetime
library such as `chrono` or `datetime`. However, the exact choice of a datetime
library may depend on the specific use case of a concrete application.

A better approach would be to define an _abstract_ time type, so that we can
implement context-generic providers that can work with _any_ time type that
the concrete context chooses. We can do this in CGP by defining _type traits_
that contain _associated types_:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

#[cgp_component {
    name: TimeTypeComponent,
    provider: ProvideTimeType,
}]
pub trait HasTimeType {
    type Time: Eq + Ord;
}

#[cgp_component {
    name: AuthTokenTypeComponent,
    provider: ProvideAuthTokenType,
}]
pub trait HasAuthTokenType {
    type AuthToken;
}
```

We first introduce a `HasTimeType` trait, which contains only an associated type
`Time`. We also have additional constraints that the abstract `Time` type must
implement `Eq` and `Ord`, so that we compare between two time values.
Similarly, we also introduce a `HasAuthTokenType` trait, which contains an `AuthToken`
associated type, but without any extra constraint.

Similar to trait methods, we can use CGP to auto derive blanket implementations
that delegate the implementation associated types to providers using `HasComponents`
and `DelegateComponent`. As such, we can use `#[cgp_component]` also on traits that
contain associated types.

With the type traits defined, we can update our authentication components to make
use of the abstract types inside the trait methods:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use std::time::Instant;
#
# use anyhow::Error;
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
#[cgp_component {
    provider: AuthTokenValidator,
}]
pub trait CanValidateAuthToken: HasAuthTokenType {
    fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
}

#[cgp_component {
    provider: AuthTokenExpiryFetcher,
}]
pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
    fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;
}

#[cgp_component {
    provider: CurrentTimeGetter,
}]
pub trait HasCurrentTime: HasTimeType {
    fn current_time(&self) -> Result<Self::Time, Error>;
}
```

The trait `CanValidateAuthToken` is updated to include `HasAuthTokenType`
as a supertrait, so that it can accept the abstract type `Self::AuthToken`
inside the method parameter `validate_auth_token`.
Similarly, `CanFetchAuthTokenExpiry` requires both `HasAuthTokenType`
and `HasTimeType`, while `HasCurrentTime` only requires `HasTimeType`.

With the abstract types in place, we can now redefine `ValidateTokenIsNotExpired`
to be implemented generically over _any_ abstract `Time` and `AuthToken` types.

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use anyhow::{anyhow, Error};
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component {
#     provider: AuthTokenValidator,
# }]
# pub trait CanValidateAuthToken: HasAuthTokenType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
# }
#
# #[cgp_component {
#     provider: AuthTokenExpiryFetcher,
# }]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
#     fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;
# }
#
# #[cgp_component {
#     provider: CurrentTimeGetter,
# }]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }
#
pub struct ValidateTokenIsNotExpired;

impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
where
    Context: HasCurrentTime + CanFetchAuthTokenExpiry,
{
    fn validate_auth_token(
        context: &Context,
        auth_token: &Context::AuthToken,
    ) -> Result<(), Error> {
        let now = context.current_time()?;

        let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

        if token_expiry < now {
            Ok(())
        } else {
            Err(anyhow!("auth token has expired"))
        }
    }
}
```

Through this example, we can see that CGP allows us to define context-generic providers
that are not only generic over the main context, but also all its associated types.
Compared to regular generic programming, instead of specifying all generic parameters
by position, we are able to parameterize the abstract types using _names_, in the form
of associated types.

## Defining Abstract Type Traits using `cgp_type!`

The type traits `HasTimeType` and `HasAuthTokenType` follows similar boilerplate,
and it may quickly become tedious as we define more abstract types. To help with
defining such type traits, the `cgp` crate provides the `cgp_type!` macro that
allows us to have much shorter definition as follows:


```rust
# extern crate cgp;
#
use cgp::prelude::*;

cgp_type!( Time: Eq + Ord );
cgp_type!( AuthToken );
```

The `cgp_type!` macro accepts the name of an abstract type `$name`, together with any
applicable constraint for that type. It then derives the same implementation as the
`cgp_component` macro, with a consumer trait named `Has{$name}Type`, a provider trait
named `Provide{$name}Type`, and a component name `${name}TypeComponent`.
Inside the traits, there is one associated type defined with `type $name: $constraints;`.

In addition to the standard derivation from `cgp_component`, `cgp_type!` also
derives some additional implementations, which we will cover the usage in later chapters.


## Trait Minimalism

At first glance, it might seem overly verbose to define multiple type traits and require
each to be explicitly included as a supertrait of a method interface. For instance,
you might be tempted to consolidate the methods and types into a single trait, like this:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use cgp::prelude::*;
# use anyhow::Error;
#
#[cgp_component {
    provider: AppImpl,
}]
pub trait AppTrait {
    type Time: Eq + Ord;

    type AuthToken;

    fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;

    fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;

    fn current_time(&self) -> Result<Self::Time, Error>;
}
```

While this approach might seem simpler, it introduces unnecessary _coupling_ between
potentially unrelated types and methods. For example, an application implementing
token validation might delegate this functionality to an external microservice.
In such a case, it is redundant to require the application to specify a Time type that
it doesn’t actually use.

In practice, we find the practical benefits of defining many _minimal_ traits often
outweight any theoretical advantages of combining multiple items into one trait.
As we will demonstrate in later chapters, having traits that contain only one type
or method would also enable more advanced CGP patterns to be applied, including
the use of `cgp_type!` that we have just covered.

We encourage readers to embrace minimal traits without concern for theoretical overhead. However, during the early phases of a project, you might prefer to consolidate items to reduce cognitive overload while learning or prototyping. As the project matures, you can always refactor and decompose larger traits into smaller, more focused ones, following the techniques outlined in this book.

## Impl-Side Associated Type Constraints

The minimalism philosophy for CGP also extends to the constraints specified
on the associated type inside a type trait.
Looking back at the definition of `HasTimeType`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
cgp_type!( Time: Eq + Ord );
```

The associated `Time` type has the constraint `Eq + Ord` specified. With this, the constraints
are imposed on _all_ concrete time types, regardless of whether they are actually used by
the providers. In fact, if we revisit our previous code, we could notice that the `Eq`
constraint is not reallying being used anywhere.

For this reason, the constraints specified on the associated type often become a bottleneck
that significantly restricts how the application can evolve. For example, as the application
grows more complex, it is not uncommon to now require `Time` to implement many additional traits,
such as `Debug + Display + Clone + Hash + Serialize + Deserialize` and so on.

Fortunately with CGP, we can reuse the same techniques as impl-side dependencies, and apply
them on the associated type constraints:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use anyhow::{anyhow, Error};
# use cgp::prelude::*;
#
cgp_type!( Time );

# cgp_type!( AuthToken );
#
# #[cgp_component {
#     provider: AuthTokenValidator,
# }]
# pub trait CanValidateAuthToken: HasAuthTokenType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
# }
#
# #[cgp_component {
#     provider: AuthTokenExpiryFetcher,
# }]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
#     fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;
# }
#
# #[cgp_component {
#     provider: CurrentTimeGetter,
# }]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }
#
pub struct ValidateTokenIsNotExpired;

impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
where
    Context: HasCurrentTime + CanFetchAuthTokenExpiry,
    Context::Time: Ord,
{
    fn validate_auth_token(
        context: &Context,
        auth_token: &Context::AuthToken,
    ) -> Result<(), Error> {
        let now = context.current_time()?;

        let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

        if token_expiry < now {
            Ok(())
        } else {
            Err(anyhow!("auth token has expired"))
        }
    }
}
```

In the above example, we redefine `HasTimeType::Time` to _not_ have any constraint.
Then in the provider implementation of `ValidateTokenIsNotExpired`, we add an
additional constraint that requires `Context::Time: Ord`. This way,
`ValidateTokenIsNotExpired` is able to compare the token expiry time, even
when `Ord` is not specified on `HasTimeType::Time`.

With this approach, we can _conditionally_ require `HasTimeType::Time` to implement
`Ord`, only when `ValidateTokenIsNotExpired` is used as the provider.
This essentially allows the abstract types to scale in the same way as the generic
context types, and allows us to make use of the same CGP patterns also on abstract types.

That said, in some cases it is still convenient to directly include constraints
such as `Debug` on an associated type, especially if the constraint is used in
almost all providers. With the current state of error reporting, including
all constraints on the associated type also tend to provide better error messages,
when there is any unsatisfied constraint.

As a guideline, we encourage readers to first try to define type traits without
including any constraint on the associated type, and try to include the constraints
on the impl-side as often as possible. However readers are free to include default
constraints to associated types as they see fit, at least for relatively trivial
types such as `Debug` and `Eq`.

## Type Providers

With type abstraction in place, we can define context-generic providers for the `Time` and `AuthToken` abstract types. For example, we can create a provider that uses `std::time::Instant` as the `Time` type:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use std::time::Instant;
#
# use cgp::prelude::*;
# use anyhow::Error;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component {
#     provider: CurrentTimeGetter,
# }]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }
#
pub struct UseInstant;

impl<Context> ProvideTimeType<Context> for UseInstant {
    type Time = Instant;
}

impl<Context> CurrentTimeGetter<Context> for UseInstant
where
    Context: HasTimeType<Time = Instant>,
{
    fn current_time(_context: &Context) -> Result<Instant, Error> {
        Ok(Instant::now())
    }
}
```

Here, the `UseInstant` provider implements `ProvideTimeType` for any `Context` type by setting the associated type `Time` to `Instant`. Additionally, it implements `CurrentTimeGetter` for any `Context`, _provided_ that `Context::Time` is `Instant`. This type equality constraint works similarly to regular implementation-side dependencies and is frequently used for scope-limited access to a concrete type associated with an abstract type.

The type equality constraint is necessary because a given context might not always use `UseInstant` as the provider for `ProvideTimeType`. Instead, the context could choose a different provider that uses another type to represent `Time`. Consequently, `UseInstant` can only implement `CurrentTimeGetter` if the `Context` uses it or another provider that also uses `Instant` as its `Time` type.

Aside from `Instant`, we can also define alternative providers for `Time`, using other types like [`datetime::LocalDateTime`](https://docs.rs/datetime/latest/datetime/struct.LocalDateTime.html):

```rust
# extern crate cgp;
# extern crate anyhow;
# extern crate datetime;
#
# use cgp::prelude::*;
# use anyhow::Error;
# use datetime::LocalDateTime;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component {
#     provider: CurrentTimeGetter,
# }]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }
#
pub struct UseLocalDateTime;

impl<Context> ProvideTimeType<Context> for UseLocalDateTime {
    type Time = LocalDateTime;
}

impl<Context> CurrentTimeGetter<Context> for UseLocalDateTime
where
    Context: HasTimeType<Time = LocalDateTime>,
{
    fn current_time(_context: &Context) -> Result<LocalDateTime, Error> {
        Ok(LocalDateTime::now())
    }
}
```

Since our application only requires the `Time` type to implement `Ord` and the ability to retrieve the current time, we can easily swap between different time providers, as long as they meet these dependencies. As the application evolves, additional constraints might be introduced on the Time type, potentially limiting the available concrete time types. However, with CGP, we can incrementally introduce new dependencies based on the application’s needs, avoiding premature restrictions caused by unused requirements.

Similarly, for the abstract `AuthToken` type, we can define a context-generic provider `ProvideAuthTokenType` that uses `String` as its implementation:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
pub struct UseStringAuthToken;

impl<Context> ProvideAuthTokenType<Context> for UseStringAuthToken {
    type AuthToken = String;
}
```

Compared to the newtype pattern, we can use plain `String` values directly, without wrapping them in a newtype struct. Contrary to common wisdom, in CGP, we place less emphasis on wrapping every domain type in a newtype. This is particularly true when most of the application is written in a context-generic style. The rationale is that abstract types and their accompanying interfaces already fulfill the role of newtypes by encapsulating and "protecting" raw values, reducing the need for additional wrapping.

That said, readers are free to define newtypes and use them alongside abstract types. For beginners, this can be especially useful, as later chapters will explore methods to properly restrict access to underlying concrete types in context-generic code. Additionally, newtypes remain valuable when the raw values are also used in non-context-generic code, where access to the concrete types is unrestricted.

Throughout this book, we will primarily use plain types to implement abstract types, without additional newtype wrapping. However, we will revisit the comparison between newtypes and abstract types in later chapters, providing further guidance on when each approach is most appropriate.

## Putting It Altogether

With all pieces in place, we can put together everything we learn, and refactor
our naive authentication components to make use of abstract types as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
# extern crate datetime;
#
# pub mod main {
pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    cgp_type!( Time );
    cgp_type!( AuthToken );

    #[cgp_component {
        provider: AuthTokenValidator,
    }]
    pub trait CanValidateAuthToken: HasAuthTokenType {
        fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
    }

    #[cgp_component {
        provider: AuthTokenExpiryFetcher,
    }]
    pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
        fn fetch_auth_token_expiry(
            &self,
            auth_token: &Self::AuthToken,
        ) -> Result<Self::Time, Error>;
    }

    #[cgp_component {
        provider: CurrentTimeGetter,
    }]
    pub trait HasCurrentTime: HasTimeType {
        fn current_time(&self) -> Result<Self::Time, Error>;
    }
}

pub mod impls {
    use anyhow::{anyhow, Error};
    use cgp::prelude::*;
    use datetime::LocalDateTime;

    use super::traits::*;

    pub struct ValidateTokenIsNotExpired;

    impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
    where
        Context: HasCurrentTime + CanFetchAuthTokenExpiry,
        Context::Time: Ord,
    {
        fn validate_auth_token(
            context: &Context,
            auth_token: &Context::AuthToken,
        ) -> Result<(), Error> {
            let now = context.current_time()?;

            let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

            if token_expiry < now {
                Ok(())
            } else {
                Err(anyhow!("auth token has expired"))
            }
        }
    }

    pub struct UseLocalDateTime;

    impl<Context> ProvideTimeType<Context> for UseLocalDateTime {
        type Time = LocalDateTime;
    }

    impl<Context> CurrentTimeGetter<Context> for UseLocalDateTime
    where
        Context: HasTimeType<Time = LocalDateTime>,
    {
        fn current_time(_context: &Context) -> Result<LocalDateTime, Error> {
            Ok(LocalDateTime::now())
        }
    }

    pub type UseStringAuthToken = UseType<String>;
}

pub mod contexts {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use cgp::prelude::*;
    use datetime::LocalDateTime;

    use super::impls::*;
    use super::traits::*;

    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, LocalDateTime>,
    }

    pub struct MockAppComponents;

    impl HasComponents for MockApp {
        type Components = MockAppComponents;
    }

    delegate_components! {
        MockAppComponents {
            [
                TimeTypeComponent,
                CurrentTimeGetterComponent,
            ]: UseLocalDateTime,
            AuthTokenTypeComponent: UseStringAuthToken,
            AuthTokenValidatorComponent: ValidateTokenIsNotExpired,
        }
    }

    impl AuthTokenExpiryFetcher<MockApp> for MockAppComponents {
        fn fetch_auth_token_expiry(
            context: &MockApp,
            auth_token: &String,
        ) -> Result<LocalDateTime, anyhow::Error> {
            context
                .auth_tokens_store
                .get(auth_token)
                .cloned()
                .ok_or_else(|| anyhow!("invalid auth token"))
        }
    }

    pub trait CanUseMockApp: CanValidateAuthToken {}

    impl CanUseMockApp for MockApp {}
}
#
# }
```

Compared to before, it is now much easier for us to update the `MockApp` context to
use different time and auth token providers. In case if we need to use different
concrete types for different use cases, we can also easily define additional
context types with different wirings, without having to duplicate the core logic.

At this point, we have make use of abstract types on the time and auth token types,
but we are still using a concrete `anyhow::Error` type. In the next chapter, we
will look into the topic of error handling, and learn how to make use of
abstract error types to better handle application errors.