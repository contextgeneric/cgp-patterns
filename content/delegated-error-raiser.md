# Delegated Error Raisers

In the previous chapter, we defined context-generic error raisers such as `RaiseFrom` and `DebugAnyhowError`, which can be used to raise any source error that satisfies certain constraints. However, in the main wiring for `MockAppComponents`, we could only select a specific provider for the `ErrorRaiserComponent`.

In more complex applications, we might want to handle different source errors in different ways, depending on the type of the source error. For example, we might use `RaiseFrom` when a `From` instance is available, and default to `DebugAnyhowError` for cases where the source error implements `Debug`.

In this chapter, we will introduce the _`UseDelegate`_ pattern, which provides a declarative approach to handle errors differently based on the source error type.

## Ad Hoc Error Raisers

One way to handle source errors differently is by defining an error raiser provider with explicit implementations for each source error. For example:

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

In this example, we define the provider `MyErrorRaiser` with explicit `ErrorRaiser` implementations for a set of source error types, assuming that the abstract `Context::Error` is `anyhow::Error`.

With explicit implementations, `MyErrorRaiser` handles different source errors in various ways. When raising a source error of type `anyhow::Error`, we simply return `e` because `Context::Error` is also `anyhow::Error`. For `Infallible`, we handle the error by matching the empty case. For `std::io::Error` and `ParseIntError`, we rely on the `From` instance, as they satisfy the constraint `core::error::Error + Send + Sync + 'static`. When raising `ErrAuthTokenHasExpired`, we use the `anyhow!` macro to format the error with the `Debug` instance. For `String` and `&'a str`, we use `anyhow!` to format the error with the `Display` instance.

While defining explicit `ErrorRaiser` implementations provides a high degree of flexibility, it also requires a significant amount of repetitive boilerplate. Since we’ve already defined various generic error raisers, it would be beneficial to find a way to _delegate_ error handling to different error raisers based on the source error type.

## `UseDelegate` Pattern

When examining the patterns for implementing custom error raisers, we notice similarities to the [provider delegation](./provider-delegation.md) pattern we covered in an earlier chapter. In fact, with a bit of indirection, we can reuse `DelegateComponent` to delegate the handling of source errors:

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

Let's walk through the code step by step. First, we define the `UseDelegate` struct with a phantom `Components` parameter. `UseDelegate` serves as a _marker_ type for implementing the trait-specific component delegation pattern. Here, we implement `ErrorRaiser` for `UseDelegate`, allowing it to act as a context-generic provider for `ErrorRaiser` under specific conditions.

Within the implementation, we specify that for any context `Context`, source error `SourceError`, and error raiser provider `Components`, `UseDelegate<Components>` implements `ErrorRaiser<Context, SourceError>` if `Components` implements `DelegateComponent<SourceError>`. Additionally, the delegate `Components::Delegate` must also implement `ErrorRaiser<Context, SourceError>`. Inside the `raise_error` method, we delegate the implementation to `Components::Delegate::raise_error`.

In simpler terms, `UseDelegate<Components>` implements `ErrorRaiser<Context, SourceError>` if there is a delegated provider `ErrorRaiser<Context, SourceError>` from `Components` via `SourceError`.

We can better understand this by looking at a concrete example. Using `UseDelegate`, we can declaratively dispatch errors as follows:

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
# pub struct DebugAnyhowError;
#
# impl<Context, E> ErrorRaiser<Context, E> for DebugAnyhowError
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
            DebugAnyhowError,
    }
}

pub type MyErrorRaiser = UseDelegate<MyErrorRaiserComponents>;
```

In this example, we first define `MyErrorRaiserComponents` and use `delegate_components!` to map source error types to the error raiser providers we wish to use. Then, we redefine `MyErrorRaiser` to be `UseDelegate<MyErrorRaiserComponents>`. This allows us to implement `ErrorRaiser` for source errors such as `std::io::Error`, `ParseIntError`, and `ErrAuthTokenHasExpired`.

We can also trace the `ErrorRaiser` implementation for `UseDelegate` and see how errors like `std::io::Error` are handled. First, `UseDelegate` implements `ErrorRaiser` because `MyErrorRaiserComponents` implements `DelegateComponent<std::io::Error>`. From there, we observe that the delegate is `RaiseFrom`, and for the case where `Context::Error` is `anyhow::Error`, a `From` instance exists for converting `std::io::Error` into `anyhow::Error`. Thus, the chain of dependencies is satisfied, and `ErrorRaiser` is implemented successfully.

As seen above, the `DelegateComponent` and `delegate_components!` constructs are not only useful for wiring up CGP providers but can also be used to dispatch providers based on the generic parameters of specific traits. In fact, we will see the same pattern applied in other contexts throughout CGP.

For this reason, the `UseDelegate` type is included in the `cgp` crate, along with the `ErrorRaiser` implementation, so that readers can easily identify when delegation is being used every time they encounter a trait implemented for `UseDelegate`.

## Forwarding Error Raiser

In addition to the delegation pattern, it can be useful to implement generic error raisers that perform a transformation on the source error and then forward the handling to another error raiser. For instance, when implementing a generic error raiser that formats the source error using `Debug`, we could first format it as a string and then forward the handling as follows:

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

In the example above, we define a generic error raiser `DebugError` that implements `ErrorRaiser` for any `SourceError` that implements `Debug`. Additionally, we require that `Context` also implements `CanRaiseError<String>`. Inside the implementation of `raise_error`, we format the source error as a string and then invoke `Context::raise_error` with the formatted string.

A forwarding error raiser like `DebugError` is designed to be used with `UseDelegate`, ensuring that the `ErrorRaiser` implementation for `String` is handled by a separate error raiser. Without this, an incorrect wiring could result in a stack overflow if `DebugError` were to call itself recursively when handling the `String` error.

The key advantage of this approach is that it remains generic over the abstract `Context::Error` type. When used correctly, this allows for a large portion of error handling to remain fully context-generic, promoting flexibility and reusability.

## Full Example

Now that we have learned how to use `UseDelegate`, we can rewrite the naive error raiser from the beginning of this chapter and use `delegate_components!` to simplify our error handling.

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

    pub struct DisplayAnyhowError;

    impl<Context, SourceError> ErrorRaiser<Context, SourceError> for DisplayAnyhowError
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
                DisplayAnyhowError,
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

In the first part of the example, we define various context-generic error raisers that are useful not only for our specific application but can also be reused later for other applications. We have `ReturnError`, which simply returns the source error as-is, `RaiseFrom` for converting the source error using `From`, `RaiseInfallible` for handling `Infallible` errors, and `DebugError` for formatting and re-raising the error as a string. We also define `UseAnyhow` to implement `ProvideErrorType`, and `DisplayAnyhowError` to convert any `SourceError` implementing `Display` into `anyhow::Error`.

In the second part of the example, we define a dummy context, `MyApp`, to illustrate how it can handle various source errors. We define `MyErrorRaiserComponents` and use `delegate_components!` to map various source error types to the corresponding error raiser providers. We then use `UseDelegate<MyErrorRaiserComponents>` as the provider for `ErrorRaiserComponent`. Finally, we define the trait `CanRaiseMyAppErrors` to verify that all the error raisers are wired correctly.

## Wiring Checks

As seen in the example, the use of `UseDelegate` with `ErrorRaiser` acts as a form of top-level error handler for an application. The main difference is that the "handling" of errors is done entirely at compile-time, enabling us to customize how each source error is handled without incurring any runtime performance overhead.

However, it's important to note that the wiring for delegated error raisers is done _lazily_, similar to how CGP provider wiring works. This means that an error could be wired incorrectly, with constraints that are not satisfied, and the issue will only manifest as a compile-time error when the error raiser is used in another provider.

Misconfigured wiring of error raisers can often lead to common CGP errors, especially for beginners. We encourage readers to refer back to the chapter on [debugging techniques](./debugging-techniques.md) and utilize check traits to ensure all source errors are wired correctly. It's also helpful to use a forked Rust compiler to display unsatisfied constraints arising from incomplete error raiser implementations.

## Conclusion

In this chapter, we explored the `UseDelegate` pattern and how it allows us to declaratively handle error raisers in various ways. This pattern simplifies error handling and can be extended to other problem domains within CGP, as we'll see in future chapters. Additionally, the `UseDelegate` pattern serves as a foundation for more advanced error handling techniques, which will be covered in the next chapter.
