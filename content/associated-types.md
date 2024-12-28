# Associated Types

In the first part of this book, we have learned about how CGP makes use of
Rust's trait system to wire up components using blanket implementations.
Since CGP works within Rust's trait system, we can make use of advanced
Rust features together with CGP to form new design patterns.
In this chapter, we will learn about how to make use of _associated types_
with CGP to define context-generic providers that are generic over multiple
types.

# Building Authentication Components

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
`CanFetchAuthTokenExpiry` used for fetching the expiry time of an auth token, if it is valid.
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

The naive example above makes use of basic CGP techniques to implement a reusable
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

Through this example, we can see what CGP allows us to define context-generic providers
that are not only generic over the main context, but also all its associated types.
Compared to regular generic programming, instead of specifying all generic parameters
by position, we are able to parameterize the abstract types using _names_, in the form
of associated types.

## Trait Minimalism

It may look overly verbose to define multiple type traits and require
the exact type trait to to be included as the supertrait of a method
interface. For example, one may be tempted to define just one trait
that contains methods and types, such as:

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

However, doing so introduces unnecessary _coupling_ between unrelated types and methods.
For example, an application may want to implement the token validation by forwarding the
validation to an external _microservice_. In such case, it would be redundant to require
the application to choose a time type that it won't actually use.

In practice, we find the practical benefits of defining many _minimal_ traits often
outweight any theoretical advantages of combining multiple items into one trait.
As we will demonstrate in later chapters, having traits that contain only one type
or method would enable more advanced CGP patterns to be applied to such traits.

Because of this, we encourage readers to follow our advice and be _encouraged_
to use as many minimal traits without worrying about any theoretical overhead.
That said, this advice is _non-binding_, so readers are free to add as many items
as they prefer into a trait, and go through the hard way of learning why the
alternative is better.

## Type Providers

With the type abstraction in place, we can define different context-generic
providers for the `Time` and `AuthToken` abstract types.
For instance, we can define a provider that provides `std::time::Instant`
as the `Time` type:

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

Our context-generic provider `UseInstant` can be used to implement
`ProvideTimeType` for any `Context` type, by setting the associated
type `Time` to be `Instant`.
Additionally, `UseInstant` also implements `CurrentTimeGetter`
for any `Context` type, _provided_ that `Context::Time` is the
same as `Instant`.
The type equality constraint works similar to how regular impl-side
dependencies work, and would be used frequently for scope-limited
access to the underlying concrete type for an abstract type.

Note that this type equality constraint is required in this case,
because a context may _not_ necessary choose `UseInstant` as
the provider for `ProvideTimeType`. As a result, there is an
additional constraint that `UseInstant` can only implement
`CurrentTimeGetter`, if `Context` uses it or a different
provider that also uses `Instant` to implement `Time`.

Aside from `Instant`, we can also implement separate time providers that
make use of a different time type, such as
[`datetime::LocalDateTime`](https://docs.rs/datetime/latest/datetime/struct.LocalDateTime.html):

```rust
# extern crate cgp;
# extern crate anyhow;
# extern crate datetime;
#
# use std::time::Instant;
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

Since our application only require `Time` to implement `Eq` and `Ord`,
and the ability to get the current time, we can easily swap between different
time providers if they satisfy all the dependencies we need.
As the application grows, there may be additional constraints imposed on
the time type, which may restrict the available choice of concrete time types.
But with CGP, we can incrementally introduce new dependencies according
the needs of the application, so that we do not prematurely restrict
our choices based on dependencies that are not used by the application.

Similar to the abstract `Time` type, we can also define a context-generic
provider for `ProvideAuthTokenType`, which implements `AuthToken` using
`String`:

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

Notice that compared to the newtype pattern, we can opt to use plain old `String`
_without_ wrapping it around a newtype struct. Contradicting to common wisdom,
we in fact do not put as much emphasis of requiring newtype wrapping every
abstract type used by the application. This is particularly the case if the
majority of the application is written as context-generic code. The reason
for this is because the abstract types and their accompanying interfaces
already serve the same purpose as newtypes, and so there are less needs
to "protect" the raw values by wrapping them inside newtypes.

That being said, readers are free to define newtypes and use them together with
abstract types. This would be helpful at least for beginners, as there are
different approaches that we will discuss in later chapters on how to properly
restrict the access of underlying concrete types inside context-generic code.
Newtypes would also still be useful, if the values are also accessed by
non-trival non-context-generic code, which would have unrestricted access to
the raw type.

In this book, we will continue using the pattern of implementing abstract types
using plain types without additional newtype wrapping. We will revisit the topic
of comparing newtypes and abstract types in later chapters.

## Putting It Altogether

```rust
# extern crate cgp;
# extern crate anyhow;
# extern crate datetime;
#
# pub mod main {
pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    #[cgp_component {
        name: TimeTypeComponent,
        provider: ProvideTimeType,
    }]
    pub trait HasTimeType {
        type Time;
    }

    #[cgp_component {
        name: AuthTokenTypeComponent,
        provider: ProvideAuthTokenType,
    }]
    pub trait HasAuthTokenType {
        type AuthToken;
    }

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

    pub struct UseStringAuthToken;

    impl<Context> ProvideAuthTokenType<Context> for UseStringAuthToken {
        type AuthToken = String;
    }
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