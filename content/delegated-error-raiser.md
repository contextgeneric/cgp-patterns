# Delegated Error Raiser

In the previous chapter, we have defined context-generic error raisers like `RaiseFrom`
and `DebugAsAnyhow`, which can be use to raise any source error that satisfy certain
constraints.
However, in the main wiring for `MockAppComponents`, we could only choose a specific
provider for `ErrorRaiserComponent`.
But with complex applications, we may want to raise different source errors differently,
depending on what the source error is.
For example, we may want to use `RaiseFrom` when there is a `From` instance, and
`DebugAsAnyhow` for the remaining cases when the source error implements `Debug`.

In this chapter, we will cover the `UseDelegate` pattern, which offers a declarative
way to handle errors differently depending on the source error type.

## Ad Hoc Error Raiser

One way that we can handle source errors differently is by defining an error raiser
provider that has explicit implementation for each source error, such as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# #[derive(Debug)]
# pub struct ErrAuthTokenHasExpired;

use core::convert::Infallible;
use core::num::ParseIntError;

use anyhow::anyhow;
use cgp::core::error::ErrorRaiser;
use cgp::prelude::*;

pub struct MyErrorRaiser;

impl<Context> ErrorRaiser<Context, anyhow::Error> for MyErrorRaiser
where
    Context: HasErrorType<Error = anyhow::Error>,
{
    fn raise_error(e: anyhow::Error) -> anyhow::Error {
        e
    }
}

impl<Context> ErrorRaiser<Context, Infallible> for MyErrorRaiser
where
    Context: HasErrorType,
{
    fn raise_error(e: Infallible) -> Context::Error {
        match e {}
    }
}

impl<Context> ErrorRaiser<Context, std::io::Error> for MyErrorRaiser
where
    Context: HasErrorType<Error = anyhow::Error>,
{
    fn raise_error(e: std::io::Error) -> anyhow::Error {
        e.into()
    }
}

impl<Context> ErrorRaiser<Context, ParseIntError> for MyErrorRaiser
where
    Context: HasErrorType<Error = anyhow::Error>,
{
    fn raise_error(e: ParseIntError) -> anyhow::Error {
        e.into()
    }
}

impl<Context> ErrorRaiser<Context, ErrAuthTokenHasExpired> for MyErrorRaiser
where
    Context: HasErrorType<Error = anyhow::Error>,
{
    fn raise_error(e: ErrAuthTokenHasExpired) -> anyhow::Error {
        anyhow!("{e:?}")
    }
}

impl<Context> ErrorRaiser<Context, String> for MyErrorRaiser
where
    Context: HasErrorType<Error = anyhow::Error>,
{
    fn raise_error(e: String) -> anyhow::Error {
        anyhow!("{e}")
    }
}

impl<'a, Context> ErrorRaiser<Context, &'a str> for MyErrorRaiser
where
    Context: HasErrorType<Error = anyhow::Error>,
{
    fn raise_error(e: &'a str) -> anyhow::Error {
        anyhow!("{e}")
    }
}
```

In the above example, we define a provider `MyErrorRaiser` that have explicit
`ErrorRaiser` implementation for a limited list of source error types, with
the assumption that the abstract `Context::Error` is instantiated to
`anyhow::Error`.

With explicit implementations, `MyErrorRaiser` is able to implement different
strategy to handle different source error.
When raising a source error `anyhow::Error`, we simply return `e` as `Context::Error`
is also `anyhow::Error`.
When raising `Infallible`, we can unconditionally handle the error by matching with
empty case.
When raising `std::io::Error` and `ParseIntError`, we can just use the `From` instance,
since they satisfy the constraint `core::error::Error + Send + Sync + 'static`.
When raising `ErrAuthTokenHasExpired`, we format the error using `anyhow!` with
the `Debug` instance.
When raising `String` and `&'a str`, we format the error using `anyhow!` with
the `Display` instance.

The approach of defining explicit `ErrorRaiser` implementations gives us a lot of
flexibility, but at the cost of requiring a lot of non-reusable boilerplate.
Given that we have previously defined various generic error raisers, it would
be good if there is a way to dispatch the error handling to different error
raiser, depending on the source error type.

## `UseDelegate` Pattern

If we look closely to the patterns of implementing custom error raisers, we would
notice that it looks similar to the [provider delegation](./provider-delegation.md)
pattern that we have went through in the earlier chapter.
In fact, with a little bit of indirection, we can reuse `DelegateComponent` to
also delegate the handling of source errors for us:

```rust
# extern crate cgp;
# extern crate anyhow;
#
use core::marker::PhantomData;

use cgp::core::error::ErrorRaiser;
use cgp::prelude::*;

pub struct UseDelegate<Components>(pub PhantomData<Components>);

impl<Context, SourceError, Components> ErrorRaiser<Context, SourceError> for UseDelegate<Components>
where
    Context: HasErrorType,
    Components: DelegateComponent<SourceError>,
    Components::Delegate: ErrorRaiser<Context, SourceError>,
{
    fn raise_error(e: SourceError) -> Context::Error {
        Components::Delegate::raise_error(e)
    }
}
```

We will walk through the code above slowly to uncover what it entails. First, we define a
`UseDelegate` struct with a `Components` phantom parameter. The type `UseDelegate` is used
as a _marker type_ for implementing trait-specific component delegation pattern.
For this case, we implement `ErrorRaiser` for `UseDelegate`, so that it can be used
as a context-generic provider for `ErrorRaiser` under specific conditions.

Inside implementation, we specify that for any context `Context`, source error `SourceError`,
and error raiser components `Components`, `UseDelegate<Components>` implements
`ErrorRaiser<Context, SourceError>` if `Components` implements `DelegateComponent<SourceError>`.
Additionally, the delegate `Components::Delegate` is also expected to implement
`ErorrRaiser<Context, SourceError>`. Inside the `raise_error` method, we simply delegate the
implementation to `Components::Delegate::raise_error`.

To explain it in simpler terms, `UseDelegate<Components>` implements `ErrorRaiser<Context, SourceError>`
if there is a delegated provider `ErrorRaiser<Context, SourceError>` that is delegated from `Components`
via `SourceError`.

We could better understand what this entails with a concrete example. Using `UseDelegate`, we can for
example declaratively dispatch errors such as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use cgp::core::component::UseDelegate;
# use cgp::core::error::ErrorRaiser;
# use cgp::prelude::*;
#
# use core::fmt::Debug;
# use core::num::ParseIntError;
#
# use anyhow::anyhow;
#
# #[derive(Debug)]
# pub struct ErrAuthTokenHasExpired;
#
# pub struct DebugAsAnyhow;
#
# impl<Context, E> ErrorRaiser<Context, E> for DebugAsAnyhow
# where
#     Context: HasErrorType<Error = anyhow::Error>,
#     E: Debug,
# {
#     fn raise_error(e: E) -> anyhow::Error {
#         anyhow!("{e:?}")
#     }
# }
#
# pub struct RaiseFrom;
#
# impl<Context, E> ErrorRaiser<Context, E> for RaiseFrom
# where
#     Context: HasErrorType,
#     Context::Error: From<E>,
# {
#     fn raise_error(e: E) -> Context::Error {
#         e.into()
#     }
# }
#
pub struct MyErrorRaiserComponents;

delegate_components! {
    MyErrorRaiserComponents {
        [
            std::io::Error,
            ParseIntError,
        ]:
            RaiseFrom,
        [
            ErrAuthTokenHasExpired,
        ]:
            DebugAsAnyhow,
    }
}

pub type MyErrorRaiser = UseDelegate<MyErrorRaiserComponents>;
```

We first define a `MyErrorRaiserComponents` type, and use `delegate_components!` on it
to map the source error type to the error raiser provider we want to use.
We then redefine `MyErrorRaiser` to be just `UseDelegate<MyErrorRaiserComponents>`.
With that, we are able to implement `ErrorRaiser` for the source errors
`std::io::Error`, `ParseIntError`, and `ErrAuthTokenHasExpired`.

Using the example, we can also trace back the `ErrorRaiser` implementation for `UseDelegate`,
and see how the handling of a source error like `std::io::Error` is wired.
First of all, `UseDelegate` implements `ErrorRaiser`, given that
`MyErrorRaiserComponents` implements `DelegateComponent<std::io::Error>`.
Following that, we can see that the `Delegate` is `RaiseFrom`, and for the case
when `Context::Error` is `anyhow::Error`, there is a `From` instance from `std::io::Error`
to `anyhow::Error`. Therefore, the chain of dependencies are satisfied, and so
the `ErrorRaiser` is implemented.

As we can see from above, the CGP constructs `DelegateComponent` and `delegate_components!`
are not only useful for wiring up CGP providers, but we can also use the same pattern
for dispatching providers based on the generic parameters of specific traits.
In fact, we will see the same pattern being used again in many other domains.

For that reason, the `UseDelegate` type is included as part of the `cgp` crate, together
with the `ErrorRaiser` implementation for it. This is so that readers can quickly identify
that the delegation is used, every time they see that a trait is implemented for `UseDelegate`.