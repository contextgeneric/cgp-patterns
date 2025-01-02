# Error Reporting

In the [previous chapter](./error-handling.md), we implemented `AuthTokenValidator`
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