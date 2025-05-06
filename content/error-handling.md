# Error Handling

Rust introduces a modern approach to error handling through the use of the `Result` type, which explicitly represents errors. Unlike implicit exceptions commonly used in other mainstream languages, the `Result` type offers several advantages. It clearly indicates when errors may occur and specifies the type of errors that might be encountered when calling a function. However, the Rust community has yet to reach a consensus on the ideal _error type_ to use within a `Result`.

Choosing an appropriate error type is challenging because different applications have distinct requirements. For instance, should the error include stack traces? Can it be compatible with no_std environments? How should the error message be presented? Should it include _structured metadata_ for introspection or specialized logging? How can different errors be distinguished to determine whether an operation should be retried? How can error sources from various libraries be composed or _flattened_ effectively? These and other concerns complicate the decision-making process.

Because of these cross-cutting concerns, discussions in the Rust community about finding a universally optimal error type are never ending. Currently, the ecosystem tends to favor libraries like [`anyhow`](https://docs.rs/anyhow) that store error values using some form of _dynamic typing_. While convenient, these approaches sacrifice some benefits of static typing, such as the ability to determine at compile time whether a function cannot produce certain errors.

CGP offers an alternative approach to error handling: using _abstract_ error types within `Result` alongside a context-generic mechanism for _raising errors_ without requiring a specific error type. In this chapter, we will explore this new approach, demonstrating how it allows error handling to be tailored to an application's precise needs.

## Abstract Error Type

In the previous chapter, we explored how to use associated types with CGP to define abstract types. Similarly to abstract types like `Time` and `AuthToken`, we can define an abstract `Error` type as follows:

```rust
# extern crate cgp;
#
use core::fmt::Debug;

use cgp::prelude::*;

#[cgp_type]
pub trait HasErrorType {
    type Error: Debug;
}
```

The `HasErrorType` trait is particularly significant because it serves as a standard type API for _all_ CGP components that involve abstract errors. Its definition is intentionally minimal, consisting of a single associated type, `Error`, constrained by `Debug` by default. This `Debug` constraint was chosen because many Rust APIs, such as `Result::unwrap`, rely on error types implementing `Debug`.

Given its ubiquity, the `HasErrorType` trait is included as part of the `cgp` crate and is available in the prelude. Therefore, we will use the version provided by `cgp` rather than redefining it locally in subsequent examples.

Building on the example from the previous chapter, we can update authentication components to leverage the abstract error type defined by `HasErrorType`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_type]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
#[cgp_component {
    provider: AuthTokenValidator,
}]
pub trait CanValidateAuthToken: HasAuthTokenType + HasErrorType {
    fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Self::Error>;
}

#[cgp_component {
    provider: AuthTokenExpiryFetcher,
}]
pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType + HasErrorType {
    fn fetch_auth_token_expiry(
        &self,
        auth_token: &Self::AuthToken,
    ) -> Result<Self::Time, Self::Error>;
}

#[cgp_component {
    provider: CurrentTimeGetter,
}]
pub trait HasCurrentTime: HasTimeType + HasErrorType {
    fn current_time(&self) -> Result<Self::Time, Self::Error>;
}
```

In these examples, each trait now includes `HasErrorType` as a supertrait, and methods return `Self::Error` in the `Result` type instead of relying on a concrete type like `anyhow::Error`. This abstraction allows greater flexibility and customization, enabling components to adapt their error handling to the specific needs of different contexts.

## Raising Errors With `From`

After adopting abstract errors in our component interfaces, the next challenge is handling these abstract errors in context-generic providers. With CGP, this is achieved by leveraging impl-side dependencies and adding constraints to the `Error` type, such as requiring it to implement `From`. This allows for the conversion of a source error into an abstract error value.

For example, we can modify the `ValidateTokenIsNotExpired` provider to convert a source error, `&'static str`, into `Context::Error` when an authentication token has expired:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_type]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component(AuthTokenValidator)]
# pub trait CanValidateAuthToken: HasAuthTokenType + HasErrorType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Self::Error>;
# }
#
# #[cgp_component(AuthTokenExpiryFetcher)]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType + HasErrorType {
#     fn fetch_auth_token_expiry(
#         &self,
#         auth_token: &Self::AuthToken,
#     ) -> Result<Self::Time, Self::Error>;
# }
#
# #[cgp_component(CurrentTimeGetter)]
# pub trait HasCurrentTime: HasTimeType + HasErrorType {
#     fn current_time(&self) -> Result<Self::Time, Self::Error>;
# }
#
#[cgp_new_provider]
impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
where
    Context: HasCurrentTime + CanFetchAuthTokenExpiry + HasErrorType,
    Context::Time: Ord,
    Context::Error: From<&'static str>
{
    fn validate_auth_token(
        context: &Context,
        auth_token: &Context::AuthToken,
    ) -> Result<(), Context::Error> {
        let now = context.current_time()?;

        let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

        if token_expiry < now {
            Ok(())
        } else {
            Err("auth token has expired".into())
        }
    }
}
```

This example demonstrates how CGP simplifies "stringy" error handling in context-generic providers by delegating the conversion from strings to concrete error values to the application. While using string errors is generally not a best practice, it is useful during the prototyping phase when precise error handling strategies are not yet established.

CGP encourages an iterative approach to error handling. Developers can begin with string errors for rapid prototyping and transition to structured error handling as the application matures. For example, we can replace the string error with a custom error type like `ErrAuthTokenHasExpired`:

```rust
# extern crate cgp;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
#
# #[cgp_type]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component(AuthTokenValidator)]
# pub trait CanValidateAuthToken: HasAuthTokenType + HasErrorType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Self::Error>;
# }
#
# #[cgp_component(AuthTokenExpiryFetcher)]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType + HasErrorType {
#     fn fetch_auth_token_expiry(
#         &self,
#         auth_token: &Self::AuthToken,
#     ) -> Result<Self::Time, Self::Error>;
# }
#
# #[cgp_component(CurrentTimeGetter)]
# pub trait HasCurrentTime: HasTimeType + HasErrorType {
#     fn current_time(&self) -> Result<Self::Time, Self::Error>;
# }
#
#[cgp_new_provider]
impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
where
    Context: HasCurrentTime + CanFetchAuthTokenExpiry + HasErrorType,
    Context::Time: Ord,
    Context::Error: From<ErrAuthTokenHasExpired>
{
    fn validate_auth_token(
        context: &Context,
        auth_token: &Context::AuthToken,
    ) -> Result<(), Context::Error> {
        let now = context.current_time()?;

        let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

        if token_expiry < now {
            Ok(())
        } else {
            Err(ErrAuthTokenHasExpired.into())
        }
    }
}

#[derive(Debug)]
pub struct ErrAuthTokenHasExpired;

impl Display for ErrAuthTokenHasExpired {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "auth token has expired")
    }
}
```

In this example, we introduced the `ErrAuthTokenHasExpired` type to represent the specific error of an expired authentication token. The `AuthTokenValidator` implementation requires `Context::Error` to implement `From<ErrAuthTokenHasExpired>` for conversion to the abstract error type. Additionally, `ErrAuthTokenHasExpired` implements both `Debug` and `Display`, allowing applications to present and log the error meaningfully.

CGP facilitates defining provider-specific error types like `ErrAuthTokenHasExpired` without burdening the provider with embedding these errors into the application's overall error handling strategy. With impl-side dependencies, constraints like `Context::Error: From<ErrAuthTokenHasExpired>` apply only when the application uses a specific provider. If an application employs a different provider to implement `AuthTokenValidator`, it does not need to handle the `ErrAuthTokenHasExpired` error.

## Raising Errors using `CanRaiseError`

In the previous section, we used the `From` constraint in the `ValidateTokenIsNotExpired` provider to raise errors such as `&'static str` or `ErrAuthTokenHasExpired`. While this approach is elegant, we quickly realize it doesn't work with common error types like `anyhow::Error`. This is because `anyhow::Error` only provides a blanket From implementation only for types that implement `core::error::Error + Send + Sync + 'static`.

This restriction is a common pain point when using error libraries like `anyhow`. The reason for this limitation is that without CGP, a type like `anyhow::Error` cannot provide multiple blanket `From` implementations without causing conflicts. As a result, using `From` can leak abstractions, forcing custom error types like `ErrAuthTokenHasExpired` to implement common traits like `core::error::Error`. Another challenge is that ownership rules prevent supporting custom `From` implementations for non-owned types like `String` and `&str`.

To address these issues, we recommend using a more flexible — though slightly more verbose—approach with CGP: the `CanRaiseError` trait, rather than relying on `From` for error conversion. Here's how we define it:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_component(ErrorRaiser)]
pub trait CanRaiseError<SourceError>: HasErrorType {
    fn raise_error(e: SourceError) -> Self::Error;
}
```

The `CanRaiseError` trait has a _generic parameter_ `SourceError`, representing the source error type that will be converted into the abstract error type `HasErrorType::Error`. By making it a generic parameter, this allows a context to raise multiple source error types and convert them into the abstract error.

Since raising errors is common in most CGP code, the `CanRaiseError` trait is included in the CGP prelude, so we don’t need to define it manually.

We can now update the `ValidateTokenIsNotExpired` provider to use `CanRaiseError` instead of `From` for error handling, raising a source error like `&'static str`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_type]
# pub trait HasTimeType {
#     type Time: Eq + Ord;
# }
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component {
#     provider: AuthTokenValidator,
# }]
# pub trait CanValidateAuthToken: HasAuthTokenType + HasErrorType {
#     fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Self::Error>;
# }
#
# #[cgp_component {
#     provider: AuthTokenExpiryFetcher,
# }]
# pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType + HasErrorType {
#     fn fetch_auth_token_expiry(
#         &self,
#         auth_token: &Self::AuthToken,
#     ) -> Result<Self::Time, Self::Error>;
# }
#
# #[cgp_component {
#     provider: CurrentTimeGetter,
# }]
# pub trait HasCurrentTime: HasTimeType + HasErrorType {
#     fn current_time(&self) -> Result<Self::Time, Self::Error>;
# }
#
#[cgp_new_provider]
impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
where
    Context: HasCurrentTime + CanFetchAuthTokenExpiry + CanRaiseError<&'static str>,
    Context::Time: Ord,
{
    fn validate_auth_token(
        context: &Context,
        auth_token: &Context::AuthToken,
    ) -> Result<(), Context::Error> {
        let now = context.current_time()?;

        let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

        if token_expiry < now {
            Ok(())
        } else {
            Err(Context::raise_error("auth token has expired"))
        }
    }
}
```

In this updated implementation, we replace the `Context: HasErrorType` constraint with `Context: CanRaiseError<&'static str>`. Since `HasErrorType` is a supertrait of `CanRaiseError`, we only need to include `CanRaiseError` in the constraint to automatically include `HasErrorType`. We also use the `Context::raise_error` method to convert the string `"auth token has expired"` into `Context::Error`.

This approach avoids the limitations of `From` and offers greater flexibility for error handling in CGP, especially when working with third-party error types like `anyhow::Error`.

## Context-Generic Error Raisers

By defining the `CanRaiseError` trait using CGP, we overcome the limitations of `From` and enable context-generic error raisers that work across various source error types. For instance, we can create a context-generic error raiser for `anyhow::Error` as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use cgp::prelude::*;
use cgp::core::error::{ErrorRaiser, ErrorRaiserComponent, HasErrorType};

#[cgp_new_provider]
impl<Context, SourceError> ErrorRaiser<Context, SourceError> for RaiseAnyhowError
where
    Context: HasErrorType<Error = anyhow::Error>,
    SourceError: core::error::Error + Send + Sync + 'static,
{
    fn raise_error(e: SourceError) -> anyhow::Error {
        e.into()
    }
}
```

Here, `RaiseAnyhowError` is a provider that implements the `ErrorRaiser` trait with generic `Context` and `SourceError`. The implementation is valid only if the `Context` implements `HasErrorType` and implements `Context::Error` as `anyhow::Error`. Additionally, the `SourceError` must satisfy `core::error::Error + Send + Sync + 'static`, which is necessary for the `From` implementation provided by `anyhow::Error`. Inside the method body, the source error is converted into `anyhow::Error` using `e.into()` since the required constraints are already satisfied.

For a more generalized approach, we can create a provider that works with _any_ error type supporting `From`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
use cgp::core::error::{ErrorRaiser, ErrorRaiserComponent, HasErrorType};

#[cgp_new_provider]
impl<Context, SourceError> ErrorRaiser<Context, SourceError> for RaiseFrom
where
    Context: HasErrorType,
    Context::Error: From<SourceError>,
{
    fn raise_error(e: SourceError) -> Context::Error {
        e.into()
    }
}
```

This implementation requires the `Context` to implement `HasErrorType` and the `Context::Error` type to implement `From<SourceError>`. With these constraints in place, this provider allows errors to be raised from any source type to `Context::Error` using `From`, without requiring explicit coupling in providers like `ValidateTokenIsNotExpired`.

The introduction of `CanRaiseError` might seem redundant when it ultimately relies on `From` in some cases. However, the purpose of this indirection is to enable _alternative_ mechanisms for converting errors when `From` is insufficient or unavailable. For example, we can define an error raiser for `anyhow::Error` that uses the `Debug` trait instead of `From`:

```rust
# extern crate cgp;
# extern crate anyhow;
#
use core::fmt::Debug;

use anyhow::anyhow;
use cgp::prelude::*;
use cgp::core::error::{ErrorRaiser, ErrorRaiserComponent, HasErrorType};

#[cgp_new_provider]
impl<Context, SourceError> ErrorRaiser<Context, SourceError> for DebugAnyhowError
where
    Context: HasErrorType<Error = anyhow::Error>,
    SourceError: Debug,
{
    fn raise_error(e: SourceError) -> anyhow::Error {
        anyhow!("{e:?}")
    }
}
```

In this implementation, the `DebugAnyhowError` provider raises any source error into an `anyhow::Error`, as long as the source error implements `Debug`. The `raise_error` method uses the `anyhow!` macro and formats the source error using the `Debug` trait. This approach allows a concrete context to use providers like `ValidateTokenIsNotExpired` while relying on `DebugAnyhowError` to raise source errors such as `&'static str` or `ErrAuthTokenHasExpired`, which only implement `Debug` or `Display`.

## The `cgp-error-anyhow` Crate

The CGP project provides the [`cgp-error-anyhow`](https://docs.rs/cgp-error-anyhow) crate, which includes the anyhow-specific providers discussed in this chapter. These constructs are offered as a separate crate rather than being part of the core `cgp` crate to avoid adding `anyhow` as a mandatory dependency.

In addition, CGP offers other error crates tailored to different error handling libraries. The [`cgp-error-eyre`](https://docs.rs/cgp-error-eyre) crate supports `eyre::Error`, while the [`cgp-error-std`](https://docs.rs/cgp-error-std) crate works with `Box<dyn core::error::Error>`.

As demonstrated in this chapter, CGP allows projects to easily switch between error handling implementations without being tightly coupled to a specific error type. For instance, if the application needs to run in a resource-constrained environment, replacing `cgp-error-anyhow` with `cgp-error-std` in the component wiring enables the application to use the simpler `Box<dyn Error>` type for error handling.

## Putting It Altogether

With the use of `HasErrorType`, `CanRaiseError`, and `cgp-error-anyhow`, we can now refactor the full example
from the previous chapter, and make it generic over the error type:

```rust
# extern crate cgp;
# extern crate cgp_error_anyhow;
# extern crate anyhow;
# extern crate datetime;
#
# pub mod main {
pub mod traits {
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
    pub trait CanValidateAuthToken: HasAuthTokenType + HasErrorType {
        fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Self::Error>;
    }

    #[cgp_component(AuthTokenExpiryFetcher)]
    pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType + HasErrorType {
        fn fetch_auth_token_expiry(
            &self,
            auth_token: &Self::AuthToken,
        ) -> Result<Self::Time, Self::Error>;
    }

    #[cgp_component(CurrentTimeGetter)]
    pub trait HasCurrentTime: HasTimeType + HasErrorType {
        fn current_time(&self) -> Result<Self::Time, Self::Error>;
    }
}

pub mod impls {
    use core::fmt::Debug;

    use anyhow::anyhow;
    use cgp::prelude::*;
    use datetime::LocalDateTime;

    use super::traits::*;

    #[derive(Debug)]
    pub struct ErrAuthTokenHasExpired;

    #[cgp_new_provider]
    impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
    where
        Context: HasCurrentTime + CanFetchAuthTokenExpiry + CanRaiseError<ErrAuthTokenHasExpired>,
        Context::Time: Ord,
    {
        fn validate_auth_token(
            context: &Context,
            auth_token: &Context::AuthToken,
        ) -> Result<(), Context::Error> {
            let now = context.current_time()?;

            let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

            if token_expiry < now {
                Ok(())
            } else {
                Err(Context::raise_error(ErrAuthTokenHasExpired))
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
        Context: HasTimeType<Time = LocalDateTime> + HasErrorType,
    {
        fn current_time(_context: &Context) -> Result<LocalDateTime, Context::Error> {
            Ok(LocalDateTime::now())
        }
    }
}

pub mod contexts {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use cgp::core::error::{ErrorRaiserComponent, ErrorTypeProviderComponent};
    use cgp::prelude::*;
    use cgp_error_anyhow::{UseAnyhowError, DebugAnyhowError};
    use datetime::LocalDateTime;

    use super::impls::*;
    use super::traits::*;

    #[cgp_context]
    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, LocalDateTime>,
    }

    delegate_components! {
        MockAppComponents {
            ErrorTypeProviderComponent:
                UseAnyhowError,
            ErrorRaiserComponent:
                DebugAnyhowError,
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

    check_components! {
        CanUseMockApp for MockApp {
            AuthTokenValidatorComponent,
        }
    }
}
#
# }
```

In the updated code, we refactored `ValidateTokenIsNotExpired` to use `CanRaiseError<ErrAuthTokenHasExpired>`, with `ErrAuthTokenHasExpired` implementing only `Debug`. Additionally, we use the provider `UseAnyhowError` from `cgp-error-anyhow`, which implements `ProvideErrorType` by setting `Error` to `anyhow::Error`.

In the component wiring for `MockAppComponents`, we wire up `ErrorTypeComponent` with `UseAnyhowError` and `ErrorRaiserComponent` with `DebugAnyhowError`. In the context-specific implementation `AuthTokenExpiryFetcher<MockApp>`, we can now use `anyhow::Error` directly, since Rust already knows that `MockApp::Error` is the same type as `anyhow::Error`.

## Conclusion

In this chapter, we provided a high-level overview of how error handling in CGP differs significantly from traditional error handling done in Rust. By utilizing abstract error types with `HasErrorType`, we can create providers that are generic over the concrete error type used by an application. The `CanRaiseError` trait allows us to implement context-generic error raisers, overcoming the limitations of non-overlapping implementations and enabling us to work with source errors that only implement traits like `Debug`.

However, error handling is a complex subject, and CGP abstractions such as `HasErrorType` and `CanRaiseError` are just the foundation for addressing this complexity. There are additional details related to error handling that we will explore in the upcoming chapters, preparing us to handle errors effectively in real-world applications.
