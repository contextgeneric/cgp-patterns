# Associated Types

In the first part of this book, we explored how CGP leverages Rust's trait system to wire up components using blanket implementations. Because CGP operates within Rust's trait system, it allows us to incorporate advanced Rust features to create new design patterns. In this chapter, we will focus on using _associated types_ with CGP to define context-generic providers that are generic over multiple _abstract_ types.

## Building Authentication Components

Suppose we want to build a simple authentication system using _bearer tokens_ with an expiration time. To achieve this, we need to fetch the expiration time of a valid token and ensure that it is not in the past. A naive approach to implementing the authentication might look like the following:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# pub mod main {
pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    #[cgp_component(AuthTokenValidator)]
    pub trait CanValidateAuthToken {
        fn validate_auth_token(&self, auth_token: &str) -> Result<(), Error>;
    }

    #[cgp_component(AuthTokenExpiryFetcher)]
    pub trait CanFetchAuthTokenExpiry {
        fn fetch_auth_token_expiry(&self, auth_token: &str) -> Result<u64, Error>;
    }

    #[cgp_component(CurrentTimeGetter)]
    pub trait HasCurrentTime {
        fn current_time(&self) -> Result<u64, Error>;
    }
}

pub mod impls {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::{anyhow, Error};
    use cgp::prelude::*;

    use super::traits::*;

    #[cgp_new_provider]
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

    #[cgp_new_provider]
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

    #[cgp_context]
    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, u64>,
    }

    delegate_and_check_components! {
        CanUseMockApp for MockApp;
        MockAppComponents {
            CurrentTimeGetterComponent: GetSystemTimestamp,
            AuthTokenValidatorComponent: ValidateTokenIsNotExpired,
        }
    }

    #[cgp_provider]
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
}
#
# }
```

In this example, we first define the `CanValidateAuthToken` trait, which serves as the primary API for validating authentication tokens. To facilitate the implementation of the validator, we also define the `CanFetchAuthTokenExpiry` trait, which is responsible for fetching the expiration time of an authentication token — assuming the token is valid. Finally, the `HasCurrentTime` trait is introduced to retrieve the current time.

Next, we define a context-generic provider, `ValidateTokenIsNotExpired`, which validates authentication tokens by comparing their expiration time with the current time. The provider fetches both the token’s expiration time and the current time, and ensure that the token is still valid. Additionally, we define another context-generic provider, `GetSystemTimestamp`, which retrieves the current time using `std::time::SystemTime::now()`.

For this demonstration, we introduce a concrete context, `MockApp`, which includes an `auth_tokens_store` field. This store is a mocked collection of authentication tokens with their respective expiration times, stored in a `BTreeMap`. We also implement the `AuthTokenExpiryFetcher` trait specifically for the `MockApp` context, which retrieves expiration times from the mocked `auth_tokens_store`. Lastly, we define the `CanUseMockApp` trait, ensuring that `MockApp` properly implements the `CanValidateAuthToken` trait through the provided wiring.

## Abstract Types

The previous example demonstrates basic CGP techniques for implementing a reusable provider, `ValidateTokenIsNotExpired`, which can work with different concrete contexts. However, the method signatures are tied to specific types. For instance, we use `String` to represent the authentication token and `u64` to represent the Unix timestamp in milliseconds.

Common practice suggests that we should use distinct types to differentiate values from different domains, reducing the chance of mixing them up. A common approach in Rust is to use the _newtype pattern_ to define wrapper types, like so:

```rust
pub struct AuthToken {
    value: String,
}

pub struct Time {
    value: u64,
}
```

While the newtype pattern helps abstract over underlying values, it doesn't fully generalize the code to work with different types. For example, instead of defining our own `Time` type with Unix timestamp semantics, we may want to use a datetime library such as `datetime` or `chrono`. The choice of library could depend on the specific use case of a concrete application.

A more flexible approach is to define an _abstract_ `Time` type that allows us to implement context-generic providers compatible with _any_ `Time` type chosen by the concrete context. This can be achieved in CGP by defining _type traits_ that contain _associated types_:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

#[cgp_component(TimeTypeProviderComponent)]
pub trait HasTimeType {
    type Time: Eq + Ord;
}

#[cgp_component(AuthTokenTypeProviderComponent)]
pub trait HasAuthTokenType {
    type AuthToken;
}
```

Here, we define the `HasTimeType` trait with an associated type `Time`, which is constrained to types that implement `Eq` and `Ord` so that they can be compared. Similarly, the `HasAuthTokenType` trait defines an associated type `AuthToken`, without any additional constraints.

Similar to regular trait methods, CGP allows us to auto-derive blanket implementations that delegate the associated types to providers using `HasCgpProvider` and `DelegateComponent`. Therefore, we can use `#[cgp_component]` on traits containing associated types as well.

With these type traits in place, we can now update our authentication components to leverage abstract types within the trait methods:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use std::time::Instant;
#
# use anyhow::Error;
# use cgp::prelude::*;
#
# #[cgp_component(TimeTypeProviderComponent)]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component(AuthTokenTypeProvider)]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
#[cgp_component(AuthTokenValidator)]
pub trait CanValidateAuthToken: HasAuthTokenType {
    fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
}

#[cgp_component(AuthTokenExpiryFetcher)]
pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
    fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;
}

#[cgp_component(CurrentTimeGetter)]
pub trait HasCurrentTime: HasTimeType {
    fn current_time(&self) -> Result<Self::Time, Error>;
}
```

Here, we modify the `CanValidateAuthToken` trait to include `HasAuthTokenType` as a supertrait, allowing it to accept the abstract type `Self::AuthToken` as a method parameter. Likewise, `CanFetchAuthTokenExpiry` requires both `HasAuthTokenType` and `HasTimeType`, while `HasCurrentTime` only requires `HasTimeType`.

With the abstract types defined, we can now update `ValidateTokenIsNotExpired` to work generically with any `Time` and `AuthToken` types:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use anyhow::{anyhow, Error};
# use cgp::prelude::*;
#
# #[cgp_component(TimeTypeProviderComponent)]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component(AuthTokenTypeProviderComponent)]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component(AuthTokenValidator)]
# pub trait CanValidateAuthToken: HasAuthTokenType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
# }
#
# #[cgp_component(AuthTokenExpiryFetcher)]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
#     fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;
# }
#
# #[cgp_component(CurrentTimeGetter)]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }
#
#[cgp_new_provider]
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

This example shows how CGP enables us to define context-generic providers that are not just generic over the context itself, but also over its associated types. Unlike traditional generic programming, where all generic parameters are specified positionally, CGP allows us to parameterize abstract types using _names_ via associated types.

## Defining Abstract Type Traits with `#[cgp_type]`

The type traits `HasTimeType` and `HasAuthTokenType` share a similar structure, and as you define more abstract types, this boilerplate can become tedious. To streamline the process, the `cgp` crate provides the `#[cgp_type]` macro, which simplifies type trait definitions.

Here's how you can define the same types with `#[cgp_type]`:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

#[cgp_type {
    provider: TimeTypeProvider,
}]
pub trait HasTimeType {
    type Time: Eq + Ord;
}

#[cgp_type {
    provider: TimeTypeProvider,
}]
pub trait HasAuthTokenType {
    type AuthToken;
}
```

The `#[cgp_type]` macro works with a CGP trait that contains a single non-generic associated type. It is an extension over `#[cgp_component]`, and generate additional constructs that make it easy to work with abstract types in CGP. When no argument is given, `#[cgp_type]` would default to generate a provider with name `{Type}TypeProvider`, and a component name `{Type}TypeProviderComponent`, where `{Type}` is the name of the associated type in the trait. So the above example can be shortened to:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

#[cgp_type]
pub trait HasTimeType {
    type Time: Eq + Ord;
}

#[cgp_type]
pub trait HasAuthTokenType {
    type AuthToken;
}
```

We will explore in a moment how using `#[cgp_type]` with a single associated type bring more convenience, as compared to alternative approaches.

## Impl-Side Associated Type Constraints

The dependency-injection capabilities of CGP opens up new choices of how to design the abstract type interfaces. Consider the earlier definition of `HasTimeType`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_type]
pub trait HasTimeType {
    type Time: Eq + Ord;
}
```

Here, the associated `Time` type is constrained by `Eq + Ord`. This means that all concrete implementations of `Time` must satisfy these constraints, regardless of whether they are actually required by the providers. In fact, if we revisit our previous examples, we notice that the `Eq` constraint isn’t used anywhere.

Such overly restrictive constraints can become a bottleneck as the application evolves. As complexity increases, it’s common to require additional traits on `Time`, such as `Debug + Display + Clone + Hash + Serialize + Deserialize` and so on. Imposing these constraints globally limits flexibility and makes it harder to adapt to changing requirements.

Fortunately, CGP allows us to apply the same principle of impl-side dependencies to associated type constraints. Consider the following example:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use anyhow::{anyhow, Error};
# use cgp::prelude::*;
#
#[cgp_type]
pub trait HasTimeType {
    type Time: Eq + Ord;
}
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component(AuthTokenValidator)]
# pub trait CanValidateAuthToken: HasAuthTokenType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
# }
#
# #[cgp_component(AuthTokenExpiryFetcher)]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
#     fn fetch_auth_token_expiry(&self, auth_token: &Self::AuthToken) -> Result<Self::Time, Error>;
# }
#
# #[cgp_component(CurrentTimeGetter)]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }

#[cgp_new_provider]
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

In this example, we redefine `HasTimeType::Time` _without_ any constraints. Instead, we specify the constraint `Context::Time: Ord` in the provider implementation for `ValidateTokenIsNotExpired`. This ensures that the `ValidateTokenIsNotExpired` provider can compare the token expiry time using `Ord`, while avoiding unnecessary global constraints on `Time`.

By applying constraints on the implementation side, we can conditionally require `HasTimeType::Time` to implement `Ord`, but only when the `ValidateTokenIsNotExpired` provider is in use. This approach allows abstract types to scale flexibly alongside generic context types, enabling the same CGP patterns to be applied to abstract types.

In some cases, it can still be convenient to include constraints (e.g., `Debug`) directly on an associated type, especially if those constraints are nearly universal across providers. Additionally, current Rust error reporting often produces clearer error messages when constraints are defined at the associated type level, as opposed to being deferred to the implementation.

Ultimately, CGP does not prevent its users from preferring one design approach over another. The minimalistic abstract type design is one that you will likely see often in CGP code, particularly in this book. However, do not hesitate to include addititional trait bounds based on your requirements and preferences!

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
# #[cgp_type]
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

#[cgp_provider]
impl<Context> TimeTypeProvider<Context> for UseInstant {
    type Time = Instant;
}

#[cgp_provider]
impl<Context> CurrentTimeGetter<Context> for UseInstant
where
    Context: HasTimeType<Time = Instant>,
{
    fn current_time(_context: &Context) -> Result<Instant, Error> {
        Ok(Instant::now())
    }
}
```

Here, the `UseInstant` provider implements `TimeTypeProvider` for any `Context` type by setting the associated type `Time` to `Instant`. Additionally, it implements `CurrentTimeGetter` for any `Context`, _provided_ that `Context::Time` is `Instant`. This type equality constraint works similarly to regular implementation-side dependencies and is frequently used for scope-limited access to a concrete type associated with an abstract type.

The type equality constraint is necessary because a given context might not always use `UseInstant` as the provider for `TimeTypeProvider`. Instead, the context could choose a different provider that uses another type to represent `Time`. Consequently, `UseInstant` can only implement `CurrentTimeGetter` if the `Context` uses it or another provider that also uses `Instant` as its `Time` type.

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
# #[cgp_type]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_component(CurrentTimeGetter)]
# pub trait HasCurrentTime: HasTimeType {
#     fn current_time(&self) -> Result<Self::Time, Error>;
# }
#
pub struct UseLocalDateTime;

#[cgp_provider]
impl<Context> TimeTypeProvider<Context> for UseLocalDateTime {
    type Time = LocalDateTime;
}

#[cgp_provider]
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

Similarly, for the abstract `AuthToken` type, we can define a context-generic provider `AuthTokenTypeProvider` that uses `String` as its implementation:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
#[cgp_new_provider]
impl<Context> AuthTokenTypeProvider<Context> for UseStringAuthToken {
    type AuthToken = String;
}
```

## Comparison to Newtype Pattern

Abstract types serve as an alternative to the newtype pattern. Compared to the newtype pattern, we can use plain `String` values directly, without wrapping them in a newtype struct. Contrary to common wisdom, in CGP, we place less emphasis on wrapping every domain type in a newtype. This is particularly true when most of the application is written in a context-generic style. The rationale is that abstract types and their accompanying interfaces already fulfill the role of newtypes by encapsulating and "protecting" raw values, reducing the need for additional wrapping.

Ultimately, there is no right or wrong whether one should use abstract types, new types, or both together. It is up to your own preference, experience, and requirements, to decide which approach is best suited for you. Just take note that abstract types will be a commonly used pattern in CGP, particularly in this book.

## The `UseType` Pattern

Implementing type providers can quickly become repetitive as the number of abstract types grows. For example, to use `String` as the `AuthToken` type, we first need to define a new struct, `UseStringAuthToken`, and then implement `AuthTokenTypeProvider` for it. To streamline this process, the `cgp_type!` macro simplifies the implementation by automatically generating a provider using the _`UseType`_ pattern. The generated implementation looks like this:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# #[cgp_component(AuthTokenTypeProvider)]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
pub struct UseType<Type>(pub PhantomData<Type>);

#[cgp_provider]
impl<Context, AuthToken> AuthTokenTypeProvider<Context> for UseType<AuthToken> {
    type AuthToken = AuthToken;
}
```

Here, `UseType` is a _marker_ type with a generic parameter `Type`, representing the type to be used for a given type trait. Since `PhantomData` is its only field, `UseType` is never intended to be used as a runtime value. The generic implementation of `AuthTokenTypeProvider` for `UseType` ensures that the AuthToken type is directly set to the `Type` parameter of `UseType`.

With this generic implementation, we can redefine `UseStringAuthToken` as a simple type alias for `UseType<String>`:

```rust
# use core::marker::PhantomData;
#
# pub struct UseType<Type>(pub PhantomData<Type>);
#
type UseStringAuthToken = UseType<String>;
```

In fact, we can even skip defining type aliases altogether and use `UseType` directly in the `delegate_components` macro when wiring type providers.

The `UseType` struct is included in the `cgp` crate, and when you define an abstract type using the `cgp_type!` macro, the corresponding generic `UseType` implementation is automatically derived. This makes `UseType` a powerful tool for simplifying component wiring and reducing boilerplate in your code.

## Putting It Altogether

With all the pieces in place, we can now apply what we've learned and refactor our naive authentication components to utilize abstract types, as shown below:

```rust
# extern crate cgp;
# extern crate anyhow;
# extern crate datetime;
#
# pub mod main {
pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    #[cgp_type]
    pub trait HasTimeType {
        type Time: Eq + Ord;
    }

    #[cgp_type]
    pub trait HasAuthTokenType {
        type AuthToken;
    }

    #[cgp_component(AuthTokenValidator)]
    pub trait CanValidateAuthToken: HasAuthTokenType {
        fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
    }

    #[cgp_component(AuthTokenExpiryFetcher)]
    pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
        fn fetch_auth_token_expiry(
            &self,
            auth_token: &Self::AuthToken,
        ) -> Result<Self::Time, Error>;
    }

    #[cgp_component(CurrentTimeGetter)]
    pub trait HasCurrentTime: HasTimeType {
        fn current_time(&self) -> Result<Self::Time, Error>;
    }
}

pub mod impls {
    use anyhow::{anyhow, Error};
    use cgp::prelude::*;
    use datetime::LocalDateTime;

    use super::traits::*;

    #[cgp_new_provider]
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

    #[cgp_provider]
    impl<Context> TimeTypeProvider<Context> for UseLocalDateTime {
        type Time = LocalDateTime;
    }

    #[cgp_provider]
    impl<Context> CurrentTimeGetter<Context> for UseLocalDateTime
    where
        Context: HasTimeType<Time = LocalDateTime>,
    {
        fn current_time(_context: &Context) -> Result<LocalDateTime, Error> {
            Ok(LocalDateTime::now())
        }
    }
}

pub mod contexts {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use cgp::prelude::*;
    use datetime::LocalDateTime;

    use super::impls::*;
    use super::traits::*;

    #[cgp_context]
    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, LocalDateTime>,
    }

    delegate_and_check_components! {
        CanUseMockApp for MockApp;
        MockAppComponents {
            [
                TimeTypeProviderComponent,
                CurrentTimeGetterComponent,
            ]: UseLocalDateTime,
            AuthTokenTypeProviderComponent: UseType<String>,
            AuthTokenValidatorComponent: ValidateTokenIsNotExpired,
        }
    }

    #[cgp_provider]
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
}
#
# }
```

Compared to our earlier approach, it is now much easier to update the `MockApp` context to use different time and auth token providers. If different use cases require distinct concrete types, we can easily define additional context types with different configurations, all without duplicating the core logic.

So far, we have applied abstract types to the `Time` and `AuthToken` types, but we are still relying on the concrete `anyhow::Error` type. In the next chapter, we will explore error handling in depth and learn how to use abstract error types to improve the way application errors are managed.
