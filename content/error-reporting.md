# Error Reporting

In the [previous chapter on error handling](./error-handling.md), we implemented `AuthTokenValidator`
to raise the error string `"auth token has expired"`, when a given auth token has expired.
Even after we defined a custom error type `ErrAuthTokenHasExpired`, it is still a dummy
struct that has a `Debug` implementation that outputs the same string
`"auth token has expired"`.
In real world applications, we know that it is good engineering practice to include
as much details to an error, so that developers and end users can more easily
diagnose the source of the problem.
On the other hand, it takes a lot of effort to properly design and show good error
messages. When doing initial development, we don't necessary want to spend too
much effort on formatting error messages, when we don't even know if the code
would survive the initial iteration.

To resolve the dilemma, developers are often forced to choose a comprehensive
error library that can do everything from error handling to error reporting.
Once the library is chosen, implementation code often becomes tightly coupled with
the error library. If there is any detail missing in the error report, it may
be challenging to include more details without diving deep into the impementation.

CGP offers better ways to resolve this dilemma, by allowing us to decouple the
logic of error handling from actual error reporting. In this chapter, we will
go into detail of how we can use CGP to improve the error report to show
more information about an expired auth token.

## Reporting Errors with Abstract Types

One challenge that CGP introduces is that with abstract types, it may be challenging
to produce good error report without knowledge about the underlying type.
We can workaround this in a naive way by using impl-side dependencies to require
the abstract types `Context::AuthToken` and `Context::Time` to implement `Debug`,
and then format them as a string before raising it as an error:

```rust
# extern crate cgp;
#
# use core::fmt::Debug;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
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
    Context: HasCurrentTime + CanFetchAuthTokenExpiry + for<'a> CanRaiseError<String>,
    Context::Time: Debug + Ord,
    Context::AuthToken: Debug,
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
            Err(Context::raise_error(
                format!(
                    "the auth token {:?} has expired at {:?}, which is earlier than the current time {:?}",
                    auth_token, token_expiry, now,
                )))
        }
    }
}
```

The example above now shows better error message. But our provider `ValidateTokenIsNotExpired` is now
tightly coupled with how the token expiry error is reported. We are now forced to implement `Debug`
for any `AuthToken` and `Time` types that we want to use. It is also not possible to customize the
error report to instead use the `Display` instance, without directly modifying the implementation
for `ValidateTokenIsNotExpired`. Similarly, we cannot easily customize how the message content is
formatted, or add additional details to the report.

## Source Error Types with Abstract Fields

To better report the error message, we would first re-introduce the `ErrAuthTokenHasExpired` source
error type that we have used in earlier examples. But now, we would also add fields with
_abstract types_ into the struct, so that it contains all values that may be essential for
generating a good error report:

```rust
# extern crate cgp;
#
# use core::fmt::Debug;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
pub struct ErrAuthTokenHasExpired<'a, Context>
where
    Context: HasAuthTokenType + HasTimeType,
{
    pub context: &'a Context,
    pub auth_token: &'a Context::AuthToken,
    pub current_time: &'a Context::Time,
    pub expiry_time: &'a Context::Time,
}
```

The `ErrAuthTokenHasExpired` struct is now parameterized by a generic lifetime `'a`
and a generic context `Context`. Inside the struct, all fields are in the form of
reference `&'a`, so that we don't perform any copy to construct the error value.
The struct has a `where` clause to require `Context` to implement `HasAuthTokenType`
and `HasTimeType`, since we need to hold their values inside the struct.
In addition to `auth_token`, `current_time`, and `expiry_time`, we also include
a `context` field with a reference to the main context, so that additional error details
may be provided through `Context`.

In addition to the struct, we also manually implement a `Debug` instance as a
default way to format `ErrAuthTokenHasExpired` as string:

```rust
# extern crate cgp;
#
# use core::fmt::Debug;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# pub struct ErrAuthTokenHasExpired<'a, Context>
# where
#     Context: HasAuthTokenType + HasTimeType,
# {
#     pub context: &'a Context,
#     pub auth_token: &'a Context::AuthToken,
#     pub current_time: &'a Context::Time,
#     pub expiry_time: &'a Context::Time,
# }
#
impl<'a, Context> Debug for ErrAuthTokenHasExpired<'a, Context>
where
    Context: HasAuthTokenType + HasTimeType,
    Context::AuthToken: Debug,
    Context::Time: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "the auth token {:?} has expired at {:?}, which is earlier than the current time {:?}",
            self.auth_token, self.expiry_time, self.current_time,
        )
    }
}
```

Inside the `Debug` instance for `ErrAuthTokenHasExpired`, we make use of impl-side dependencies
to require `Context::AuthToken` and `Context::Time` to implement `Debug`. We then use `Debug`
to format the values and show the error message.

Notice that even though `ErrAuthTokenHasExpired` contains a `context` field, it is not used
in the `Debug` implementation. Also, since the `Debug` constraint for `Context::AuthToken` and
`Context::Time` are only present in the `Debug` implementation, it is possible for the concrete
types to not implement `Debug`, _if_ the application do not use `Debug` with `ErrAuthTokenHasExpired`.

This design is intentional, as we only provide the `Debug` implementation as a _convenience_
for quickly formatting the error message without further customization.
On the other hand, a better error reporting strategy may be present elsewhere and provided
by the application.
The main purpose of this design is so that at the time `ErrAuthTokenHasExpired` and
`ValidateTokenIsNotExpired` are defined, we don't need to concern about where and how
this error reporting strategy is implemented.

Using the new `ErrAuthTokenHasExpired`, we can now re-implement `ValidateTokenIsNotExpired`
as follows:

```rust
# extern crate cgp;
#
# use core::fmt::Debug;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
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
# pub struct ErrAuthTokenHasExpired<'a, Context>
# where
#     Context: HasAuthTokenType + HasTimeType,
# {
#     pub context: &'a Context,
#     pub auth_token: &'a Context::AuthToken,
#     pub current_time: &'a Context::Time,
#     pub expiry_time: &'a Context::Time,
# }
#
# impl<'a, Context> Debug for ErrAuthTokenHasExpired<'a, Context>
# where
#     Context: HasAuthTokenType + HasTimeType,
#     Context::AuthToken: Debug,
#     Context::Time: Debug,
# {
#     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
#         write!(
#             f,
#             "the auth token {:?} has expired at {:?}, which is earlier than the current time {:?}",
#             self.auth_token, self.expiry_time, self.current_time,
#         )
#     }
# }
#
pub struct ValidateTokenIsNotExpired;

impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
where
    Context: HasCurrentTime
        + CanFetchAuthTokenExpiry
        + for<'a> CanRaiseError<ErrAuthTokenHasExpired<'a, Context>>,
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
            Err(Context::raise_error(ErrAuthTokenHasExpired {
                context,
                auth_token,
                current_time: &now,
                expiry_time: &token_expiry,
            }))
        }
    }
}
```

In the new implementation, we include the constraint
`for<'a> CanRaiseError<ErrAuthTokenHasExpired<'a, Context>>`
with [_higher ranked trait bound_](https://doc.rust-lang.org/nomicon/hrtb.html),
so that we can raise `ErrAuthTokenHasExpired` parameterized with any lifetime.
Notice that inside the `where` constraints, we no longer require the `Debug`
bound on `Context::AuthToken` and `Context::Time`.

With this approach, we have made use of `ErrAuthTokenHasExpired` to fully
decouple `ValidateTokenIsNotExpired` provider from the problem of how to report
the token expiry error.

## Error Report Raisers

In the [previous chapter](./delegated-error-raiser.md), we have learned about
how to define custom error raisers and then dispatch them using the `UseDelegate`
pattern. With that in mind, we can easily define error raisers for
`ErrAuthTokenHasExpired` to format it in different ways.

One thing to note is that since `ErrAuthTokenHasExpired` contains a lifetime
parameter with borrowed values, any error raiser that handles it would
likely have to make use of the borrowed value to construct an owned value
for `Context::Error`.

The simplest way to raise `ErrAuthTokenHasExpired` is to make use of its `Debug`
implementation to and raise it using `DebugError`:

```rust
# extern crate cgp;
#
use cgp::core::error::{CanRaiseError, ErrorRaiser};
use core::fmt::Debug;

pub struct DebugError;

impl<Context, SourceError> ErrorRaiser<Context, SourceError> for DebugError
where
    Context: CanRaiseError<String>,
    SourceError: Debug,
{
    fn raise_error(e: SourceError) -> Context::Error {
        Context::raise_error(format!("{e:?}"))
    }
}
```

As we discussed in the previous chapter, `DebugError` would implement `ErrorRaiser`
if `ErrAuthTokenHasExpired` implements `Debug`. But recall that the `Debug` implementation
for `ErrAuthTokenHasExpired` requires both `Context::AuthToken` and `Context::Time` to
implement `Debug`. So in a way, the use of impl-side dependencies here is _deeply nested_,
but nevertheless still works thanks to Rust's trait system.

Now supposed that instead of using `Debug`, we want to use the `Display` instance of
`Context::AuthToken` and `Context::Time` to format the error. Even if we are in a crate
that do not own `ErrAuthTokenHasExpired`, we can still implement a custom `ErrorRaiser`
instance as follows:

```rust
# extern crate cgp;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
# use cgp::core::error::ErrorRaiser;
#
# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# pub struct ErrAuthTokenHasExpired<'a, Context>
# where
#     Context: HasAuthTokenType + HasTimeType,
# {
#     pub context: &'a Context,
#     pub auth_token: &'a Context::AuthToken,
#     pub current_time: &'a Context::Time,
#     pub expiry_time: &'a Context::Time,
# }
#
pub struct DisplayAuthTokenExpiredError;

impl<'a, Context> ErrorRaiser<Context, ErrAuthTokenHasExpired<'a, Context>>
    for DisplayAuthTokenExpiredError
where
    Context: HasAuthTokenType + HasTimeType + CanRaiseError<String>,
    Context::AuthToken: Display,
    Context::Time: Display,
{
    fn raise_error(e: ErrAuthTokenHasExpired<'a, Context>) -> Context::Error {
        Context::raise_error(format!(
            "the auth token {} has expired at {}, which is earlier than the current time {}",
            e.auth_token, e.expiry_time, e.current_time,
        ))
    }
}
```

With this approach, we can now use `DisplayAuthTokenExpiredError` if `Context::AuthToken`
and `Context::Time` implement `Display`. But even if they don't, we are still free to choose
alternative strategies for our application.

One possible way to improve the error message is to obfuscate the auth token, so that the
reader of the error message cannot know about the actual auth token. This may have already
been done, if the concrete `AuthToken` type implements a custom `Display` that does so.
But in case if it does not, we can still do something similar using a customized error raiser:


```rust
# extern crate cgp;
# extern crate sha1;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
# use cgp::core::error::ErrorRaiser;
use sha1::{Digest, Sha1};

# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# pub struct ErrAuthTokenHasExpired<'a, Context>
# where
#     Context: HasAuthTokenType + HasTimeType,
# {
#     pub context: &'a Context,
#     pub auth_token: &'a Context::AuthToken,
#     pub current_time: &'a Context::Time,
#     pub expiry_time: &'a Context::Time,
# }
#
pub struct ShowAuthTokenExpiredError;

impl<'a, Context> ErrorRaiser<Context, ErrAuthTokenHasExpired<'a, Context>>
    for ShowAuthTokenExpiredError
where
    Context: HasAuthTokenType + HasTimeType + CanRaiseError<String>,
    Context::AuthToken: Display,
    Context::Time: Display,
{
    fn raise_error(e: ErrAuthTokenHasExpired<'a, Context>) -> Context::Error {
        let auth_token_hash = Sha1::new_with_prefix(e.auth_token.to_string()).finalize();

        Context::raise_error(format!(
            "the auth token {:x} has expired at {}, which is earlier than the current time {}",
            auth_token_hash, e.expiry_time, e.current_time,
        ))
    }
}
```

By decoupling the error reporting from the provider, we can now customize the error reporting
as we see fit, without needing to access or modify the original provider `ValidateTokenIsNotExpired`.

## Context-Specific Error Details

Previously, we included the `context` field in `ErrAuthTokenHasExpired` but never used it in
the error reporting. But with the ability to define custom error raisers, we can also
define one that extracts additional details from the context, so that it can be included
in the error message.

Supposed that we are using `CanValidateAuthToken` in an application that serves sensitive documents.
When an expired auth token is used, we may want to also include the document ID being accessed,
so that we can identify the attack patterns of any potential attacker.
If the application context holds the document ID, we can now access it within the error raiser
as follows:


```rust
# extern crate cgp;
# extern crate sha1;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
# use cgp::core::error::ErrorRaiser;
use sha1::{Digest, Sha1};

# #[cgp_component {
#     name: TimeTypeProviderComponent,
#     provider: TimeTypeProvider,
# }]
# pub trait HasTimeType {
#     type Time;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeProviderComponent,
#     provider: AuthTokenTypeProvider,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# pub struct ErrAuthTokenHasExpired<'a, Context>
# where
#     Context: HasAuthTokenType + HasTimeType,
# {
#     pub context: &'a Context,
#     pub auth_token: &'a Context::AuthToken,
#     pub current_time: &'a Context::Time,
#     pub expiry_time: &'a Context::Time,
# }
#
#[cgp_component {
    provider: DocumentIdGetter,
}]
pub trait HasDocumentId {
    fn document_id(&self) -> u64;
}

pub struct ShowAuthTokenExpiredError;

impl<'a, Context> ErrorRaiser<Context, ErrAuthTokenHasExpired<'a, Context>>
    for ShowAuthTokenExpiredError
where
    Context: HasAuthTokenType + HasTimeType + CanRaiseError<String> + HasDocumentId,
    Context::AuthToken: Display,
    Context::Time: Display,
{
    fn raise_error(e: ErrAuthTokenHasExpired<'a, Context>) -> Context::Error {
        let document_id = e.context.document_id();
        let auth_token_hash = Sha1::new_with_prefix(e.auth_token.to_string()).finalize();

        Context::raise_error(format!(
            "failed to access highly sensitive document {} at time {}, using the auth token {:x} which was expired at {}",
            document_id, e.current_time, auth_token_hash, e.expiry_time,
        ))
    }
}
```

With this, even though the provider `ValidateTokenIsNotExpired` did not know that `Context` contains
a document ID, by including the `context` value in `ErrAuthTokenHasExpired`, we can
still implement a custom error raiser that produce a custom error message that includes the document ID.

## Conclusion

In this chapter, we have learned about some advanced CGP techniques that can be used to decouple providers
from the burden of producing good error reports. With that, we are able to define custom error raisers
that produce highly detailed error reports, without needing to modify the original provider implementation.
The use of source error types with abstract fields and borrowed values serves as a cheap interface to decouple
the producer of an error (the provider) from the handler of an error (the error raiser).

Still, even with CGP, learning all the best practices of properly raising and handling errors can be overwhelming,
especially for beginners. Furthermore, even if we can decouple and customize the handling of all possible error
cases, extra effort is still needed for every customization, which can still takes a lot of time.

As a result, we do not encourage readers to try and define custom error structs for all
possible errors. Instead, readers should start with simple error types like strings, and slowly add more structures
to common errors that occur in the application.
But readers should keep in mind the techniques introduced in this chapter, so that by the time we need to
customize and produce good error reports for our applications, we know about how this can be done using CGP.
