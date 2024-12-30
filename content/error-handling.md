# Error Handling

Rust provides a relatively new way of handling errors, with the use of `Result` type
to represent explicit errors. Compared to the practice of implicit exceptions in other
mainstream languages, the explicit `Result` type provides many advantages, such as
making it clear when and what kind of errors can occur when calling a function.
However, until now there is not yet a clear consensus of which _error type_ should
be used within a `Result`.

The reason why choosing an error type is complicated is often due to different
applications having different concerns: Should the error capture stack traces?
Can the error be used in no_std environment? How should the error message be
displayed? Should the error contain _structured metadata_ that can be introspected
or logged differently? How should one differentiate different errors to decide
whether to retry an operation? How to compose or _flatten_ error sources that
come from using different libraries? etc.

Due to the complex cross-cutting concerns, there are never-ending discussions
across the Rust communities on the quest to find a perfect error type that
can be used to solve _all_ error handling problems. At the moment, the
Rust ecosystem leans toward using error libraries such as
[`anyhow`](https://docs.rs/anyhow) to store error values using some
form of _dynamic typing_. However, these approaches give up some of the
advantages provided by static types, such as the ability to statically
know whether a function would never raise certain errors.

CGP offers us an alternative approach towards error handling, which is
to use _abstract_ error types in `Result`, together with a context-generic
way of _raising errors_ without access to the concrete type.
In this chapter, we will walk through this new approach of error handling,
and look at how it allows error handling to be easily customized depending
on the exact needs of an application.

## Abstract Error Type

In the previous chapter, we have learned about how to use associated types
together with CGP to define abstract types.
Similar to the abstract `Time` and `AuthToken` types, we can define an abstract
`Error` type as follows:

```rust
# extern crate cgp;
#
use core::fmt::Debug;

use cgp::prelude::*;

#[cgp_component {
    name: ErrorTypeComponent,
    provider: ProvideErrorType,
}]
pub trait HasErrorType {
    type Error: Debug;
}
```

The trait `HasErrorType` is quite special, in the sense that it serves as a standard
type API for _all_ CGP components that make use of some form of abstract errors.
Because of this, it has a pretty minimal definition, having an associated type
`Error` with a default `Debug` constraint. We chose to require the `Debug` constraint
for abstract errors, because many Rust APIs such as `Result::unwrap` already
expect error types to implement `Debug`.

The use for `HasErrorType` is so common, that it is included as part of the `cgp` crate,
and is included in the prelude. So moving forward, we will import the `HasErrorType` trait
from `cgp`, instead of defining it locally.

Continuing from the example in the previous chapter, we can update authentication components
to use the abstract error type provided by `HasErrorType`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time;
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

Each of the traits now include `HasErrorType` as a supertrait, and the methods now
return `Self::Error` instead of `anyhow::Error` in the returned `Result`.

## Raising Errors With `From`

Now that we have made use of abstract errors over concrete errors in our component interfaces,
a challenge that arise next is how can we raise abstract errors inside our context-generic
providers.
With CGP, we can make use of impl-side dependencies as usual, and include additional constraints
on the `Error` type, such as requiring it to implement `From` to convert a low-level error into
an abstract error value.

Using this technique, we can re-write `ValidateTokenIsNotExpired` to convert a `&'static str`
into `Context::Error`, when an auth token has expired:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time;
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
pub struct ValidateTokenIsNotExpired;

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

As we can see from the example, CGP makes it easy to make use of "stringy" error handling
inside context-generic providers, by offloading the task of converting from strings to the
actual error value to the concrete application.
Although the use of strings as error is not exactly a good practice, it can be very helpful
during rapid prototyping phase, when we don't yet care about how exactly we want to handle
various errors.

With CGP, we want to enable an iterative approach, where developers can make the choise to
use stringly errors in the early stage, and then gradually transition toward more structured
error handling at later stages of development.
For example, at a later time, we could replace the string error with a custom
`ErrAuthTokenHasExpired` as follows:

```rust
# extern crate cgp;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeComponent,
#     provider: ProvideTimeType,
# }]
# pub trait HasTimeType {
#     type Time;
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
pub struct ValidateTokenIsNotExpired;

#[derive(Debug)]
pub struct ErrAuthTokenHasExpired;

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

impl Display for ErrAuthTokenHasExpired {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "auth token has expired")
    }
}
```

Compared to before, we defined an `ErrAuthTokenHasExpired` type to represent the
error that happens when an auth token has expired.
Inside `AuthTokenValidator`, we now require `Context::Error` to implement `From<ErrAuthTokenHasExpired>`
to convert an expired token error into the abstract error.
The type `ErrAuthTokenHasExpired` implements both `Debug` and `Display`, so that the application
may use them when converting into `Context::Error`.

CGP makes it easy to define provider-specific error types such as `ErrAuthTokenHasExpired`,
without requiring the provider to worry about how to embed that error within the application
error as a whole.
With impl-side dependencies, an extra constraint like `Context::Error: From<ErrAuthTokenHasExpired>`
would only be applicable if the application choose to use the specific provider.
This also means that if an application chose a provider other than `ValidateTokenIsNotExpired`
to implement `AuthTokenValidator`, then it would not need to handle the error `ErrAuthTokenHasExpired`.

## Raising Errors using `CanRaiseError`

In the previous section, we used the `From` constraint in the provider implementation of
`ValidateTokenIsNotExpired` to raise either `&'static str` or `ErrAuthTokenHasExpired`.
Although this approach looks elegant, we would quickly realized that this approach
would _not_ work with popular error types such as `anyhow::Error`.
This is because `anyhow::Error` only provide a blanket `From` instance for types
that `core::error::Error + Send + Sync + 'static`.

This restriction is a common pain point when using error libraries like `anyhow`.
But the restriction is there because without CGP, a type like `anyhow::Error`
cannot provide other blanket implementations for `From` as it would cause overlap.
The use of `From` also causes leaky abstraction, as custom error types like
`ErrAuthTokenHasExpired` are forced to anticipate the use and implement the
common constraints like `core::error::Error`.
Furthermore, the ownership rules also make it impossible to support custom `From`
implementations for non-owned types, such as `String` and `&str`.

For these reasons, we don't actually encourage the use of `From` for conversion
into abstract errors. Instead, with CGP we prefer the use of a more flexible,
albeit more verbose approach, which is to use the `CanRaiseError` trait:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_component {
    provider: ErrorRaiser
}]
pub trait CanRaiseError<E>: HasErrorType {
    fn raise_error(e: E) -> Self::Error;
}
```