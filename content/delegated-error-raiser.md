# Delegated Error Raisers

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

## Ad Hoc Error Raisers

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

## Forwarding Error Raiser

Aside form the delegation pattern, it can also be useful to implement generic error raisers
that perform some transformation of the source error, and then forward the handling to another
error raiser. For example, when implementing a generic error raiser that uses `Debug` on
the source error, we could first format it and then raise it as a string as follows:

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

In the example above, we define a generic error raiser `DebugError`, which implements
`ErrorRaiser` for any `SourceError` that implements `Debug`.
Additionally, we also require that `Context` implements `CanRaiseError<String>`.
Inside the implementation of `raise_error`, we simply format the source error as
a string, and then call `Context::raise_error` again on the formatted string.

A forwarding error raiser like `DebugError` is inteded to be used together with
`UseDelegate`, so that the `ErrorRaiser` implementation of `String` is expected
to be handled by a concrete error raiser. Otherwise, an incorrect wiring may
result in a stack overflow, if `DebugError` ended up calling itself again
to handle the error raising of `String`.

Nevertheless, the main advantage for this definition is that it is also generic
over the abstract `Context::Error` type. So when used carefully, we can keep a lot
of error handling code fully context-generic this way.

## Full Example

Now that we have learned about how to use `UseDelegate`, we can rewrite the naive
error raiser that we defined in the beginning of this chapter, and use `delegate_components!`
to simplify our error handling.

```rust
# extern crate cgp;
# extern crate anyhow;
#
# pub mod main {
pub mod impls {
    use core::convert::Infallible;
    use core::fmt::{Debug, Display};

    use anyhow::anyhow;
    use cgp::core::error::{CanRaiseError, ErrorRaiser, ProvideErrorType};
    use cgp::prelude::HasErrorType;

    #[derive(Debug)]
    pub struct ErrAuthTokenHasExpired;

    pub struct ReturnError;

    impl<Context, Error> ErrorRaiser<Context, Error> for ReturnError
    where
        Context: HasErrorType<Error = Error>,
    {
        fn raise_error(e: Error) -> Error {
            e
        }
    }

    pub struct RaiseFrom;

    impl<Context, SourceError> ErrorRaiser<Context, SourceError> for RaiseFrom
    where
        Context: HasErrorType,
        Context::Error: From<SourceError>,
    {
        fn raise_error(e: SourceError) -> Context::Error {
            e.into()
        }
    }

    pub struct RaiseInfallible;

    impl<Context> ErrorRaiser<Context, Infallible> for RaiseInfallible
    where
        Context: HasErrorType,
    {
        fn raise_error(e: Infallible) -> Context::Error {
            match e {}
        }
    }

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

    pub struct UseAnyhow;

    impl<Context> ProvideErrorType<Context> for UseAnyhow {
        type Error = anyhow::Error;
    }

    pub struct DisplayAsAnyhow;

    impl<Context, SourceError> ErrorRaiser<Context, SourceError> for DisplayAsAnyhow
    where
        Context: HasErrorType<Error = anyhow::Error>,
        SourceError: Display,
    {
        fn raise_error(e: SourceError) -> anyhow::Error {
            anyhow!("{e}")
        }
    }
}

pub mod contexts {
    use core::convert::Infallible;
    use core::num::ParseIntError;

    use cgp::core::component::UseDelegate;
    use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
    use cgp::prelude::*;

    use super::impls::*;

    pub struct MyApp;

    pub struct MyAppComponents;

    pub struct MyErrorRaiserComponents;

    impl HasComponents for MyApp {
        type Components = MyAppComponents;
    }

    delegate_components! {
        MyAppComponents {
            ErrorTypeComponent: UseAnyhow,
            ErrorRaiserComponent: UseDelegate<MyErrorRaiserComponents>,
        }
    }

    delegate_components! {
        MyErrorRaiserComponents {
            anyhow::Error: ReturnError,
            Infallible: RaiseInfallible,
            [
                std::io::Error,
                ParseIntError,
            ]:
                RaiseFrom,
            [
                ErrAuthTokenHasExpired,
            ]:
                DebugError,
            [
                String,
                <'a> &'a str,
            ]:
                DisplayAsAnyhow,
        }
    }

    pub trait CanRaiseMyAppErrors:
        CanRaiseError<anyhow::Error>
        + CanRaiseError<Infallible>
        + CanRaiseError<std::io::Error>
        + CanRaiseError<ParseIntError>
        + CanRaiseError<ErrAuthTokenHasExpired>
        + CanRaiseError<String>
        + for<'a> CanRaiseError<&'a str>
    {
    }

    impl CanRaiseMyAppErrors for MyApp {}
}
# }
```

In the first part of the above example, we define various context-generic error raisers
that are not only useful for our specific application, but can also be reused later for
other applications. We have `ReturnError` which returns the source error as is,
`RaiseFrom` to use `From` to convert the source error, `RaiseInfallible` to unconditionally
match `Infallible`, and `DebugError` to format and re-raise the error as string.
We also define `UseAnyhow` to implement `ProvideErrorType`, and `DisplayAsAnyhow`
to convert any `SourceError` implementing `Display` to `anyhow::Error`.

In the second part of the example, we define a dummy context `MyApp` with the only purpose
is to show how it can handle various source errors. We define `MyErrorRaiserComponents`,
and use `delegate_components!` to map various source error types to use the error
raiser provider that we want to designate. We then use `UseDelegate<MyErrorRaiserComponents>`
as the provider for `ErrorRaiserComponent`. Finally, we define a check trait
`CanRaiseMyAppErrors`, and verify that the wiring for all error raisers are working correctly.

## Wiring Checks

As we can see from the example, the use of `UseDelegate` with `ErrorRaiser` effectively
serves as something similar to a top-level error handler for an application.
The main difference is that this "handling" of error is done entirely at compile-time.
This allows us to easily customize how exactly we want to handle each source error in
our application, and not pay for any performance overhead to achieve this level of
customization.

One thing to note however, is that the wiring for delegated error raisers is done _lazily_,
similar to how the wiring is done for CGP providers. As a result, we may incorrectly wire
a source error type to use an error raiser provider with unsatisfied constraints, and only
get a compile-time error later on when the error raiser is used in another provider.

Because of this, having misconfigured wiring of error raisers can be a common source of CGP
errors, especially for beginners.
We would encourage readers to revisit the chapter on [debugging techniques](./debugging-techniques.md)
and use the check traits to ensure that the handling of all source errors are wired correctly.
It would often helps to use the forked Rust compiler, to show the unsatisfied constraints
that arise from incomplete error raiser implementations.

## Conclusion

In this chapter, we have learned about using the `UseDelegate` pattern to declaratively
handle the error raisers in different ways.
As we will see in future chapters, the `UseDelegate` can also be applied to many other
problem domains in CGP.
The pattern is also essential for us to apply more advanced error handling techniques,
which we will cover in the next chapter.