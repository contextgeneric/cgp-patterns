# Error Wrapping

When programming in Rust, there is a common need to not only raise new errors, but also
attach additional details to an error that has previously been raised.
This is mainly to allow a caller to attach additional details about which higher-level
operations are being performed, so that better error report and diagnostics can be
presented to the user.

Error libraries such as `anyhow` and `eyre` provide methods such as
[`context`](https://docs.rs/anyhow/latest/anyhow/struct.Error.html#method.context) and
[`wrap_err`](https://docs.rs/eyre/latest/eyre/struct.Report.html#method.wrap_err)
to allow wrapping of additional details to their error type.
In this chapter, we will discuss about how to implement context-generic error wrapping
with CGP, and how to integrate them with existing error libraries.

## Example: Config Loader

Supposed that we want to build an application with the functionality to load and parse
some application configuration from a config path. Using the CGP patterns that we have
learned so far, we may implement a context-generic config loader as follows:

```rust
# extern crate cgp;
# extern crate serde;
# extern crate serde_json;
#
# pub mod main {
pub mod traits {
    use std::path::PathBuf;

    use cgp::prelude::*;

    #[cgp_component {
        name: ConfigTypeComponent,
        provider: ProvideConfigType,
    }]
    pub trait HasConfigType {
        type Config;
    }

    #[cgp_component {
        provider: ConfigLoader,
    }]
    pub trait CanLoadConfig: HasConfigType + HasErrorType {
        fn load_config(&self) -> Result<Self::Config, Self::Error>;
    }

    #[cgp_component {
        provider: ConfigPathGetter,
    }]
    pub trait HasConfigPath {
        fn config_path(&self) -> &PathBuf;
    }
}

pub mod impls {
    use std::{fs, io};

    use cgp::core::error::{ErrorRaiser, ProvideErrorType};
    use cgp::prelude::*;
    use serde::Deserialize;

    use super::traits::*;

    pub struct LoadJsonConfig;

    impl<Context> ConfigLoader<Context> for LoadJsonConfig
    where
        Context: HasConfigType
            + HasConfigPath
            + CanRaiseError<io::Error>
            + CanRaiseError<serde_json::Error>,
        Context::Config: for<'a> Deserialize<'a>,
    {
        fn load_config(context: &Context) -> Result<Context::Config, Context::Error> {
            let config_path = context.config_path();

            let config_bytes = fs::read(config_path).map_err(Context::raise_error)?;

            let config = serde_json::from_slice(&config_bytes).map_err(Context::raise_error)?;

            Ok(config)
        }
    }
}
# }
```

We first define the `HasConfigType` trait, which provides an abstract `Config` type
to represent the application's config.
We then define a `CanLoadConfig` trait, which provides an interface for loading
the application config.
To help with the implementation, we also implement a `HasConfigPath` trait,
which allows a provider to get the file path to the config file from the context.

Using the config traits, we then implement `LoadJsonConfig` as a context-generic
provider for `ConfigLoader`, which would read a JSON config file as bytes from the
filesystem using `std::fs::read`, and then parse the config using `serde_json`.
With CGP, `LoadJsonConfig` can work with any `Config` type that implements `Deserialize`.

We can then define an example application context that makes use of `LoadJsonConfig` to
load its config as follows:

```rust
# extern crate anyhow;
# extern crate cgp;
# extern crate serde;
# extern crate serde_json;
#
# pub mod main {
# pub mod traits {
#     use std::path::PathBuf;
#
#     use cgp::prelude::*;
#
#     #[cgp_component {
#         name: ConfigTypeComponent,
#         provider: ProvideConfigType,
#     }]
#     pub trait HasConfigType {
#         type Config;
#     }
#
#     #[cgp_component {
#         provider: ConfigLoader,
#     }]
#     pub trait CanLoadConfig: HasConfigType + HasErrorType {
#         fn load_config(&self) -> Result<Self::Config, Self::Error>;
#     }
#
#     #[cgp_component {
#         provider: ConfigPathGetter,
#     }]
#     pub trait HasConfigPath {
#         fn config_path(&self) -> &PathBuf;
#     }
# }
#
# pub mod impls {
#     use std::{fs, io};
#
#     use cgp::core::error::{ErrorRaiser, ProvideErrorType};
#     use cgp::prelude::*;
#     use serde::Deserialize;
#
#     use super::traits::*;
#
#     pub struct LoadJsonConfig;
#
#     impl<Context> ConfigLoader<Context> for LoadJsonConfig
#     where
#         Context: HasConfigType
#             + HasConfigPath
#             + CanRaiseError<io::Error>
#             + CanRaiseError<serde_json::Error>,
#         Context::Config: for<'a> Deserialize<'a>,
#     {
#         fn load_config(context: &Context) -> Result<Context::Config, Context::Error> {
#             let config_path = context.config_path();
#
#             let config_bytes = fs::read(config_path).map_err(Context::raise_error)?;
#
#             let config = serde_json::from_slice(&config_bytes).map_err(Context::raise_error)?;
#
#             Ok(config)
#         }
#     }
#
#     pub struct UseAnyhowError;
#
#     impl<Context> ProvideErrorType<Context> for UseAnyhowError {
#         type Error = anyhow::Error;
#     }
#
#     pub struct RaiseFrom;
#
#     impl<Context, SourceError> ErrorRaiser<Context, SourceError> for RaiseFrom
#     where
#         Context: HasErrorType,
#         Context::Error: From<SourceError>,
#     {
#         fn raise_error(e: SourceError) -> Context::Error {
#             e.into()
#         }
#     }
# }

pub mod contexts {
    use std::io;
    use std::path::PathBuf;

    use cgp::core::component::UseDelegate;
    use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
    use cgp::prelude::*;
    use serde::Deserialize;

    use super::impls::*;
    use super::traits::*;

    pub struct App {
        pub config_path: PathBuf,
    }

    #[derive(Deserialize)]
    pub struct AppConfig {
        pub api_secret: String,
    }

    pub struct AppComponents;

    pub struct RaiseAppErrors;

    impl HasComponents for App {
        type Components = AppComponents;
    }

    delegate_components! {
        AppComponents {
            ErrorTypeComponent: UseAnyhowError,
            ErrorRaiserComponent: UseDelegate<RaiseAppErrors>,
            ConfigLoaderComponent: LoadJsonConfig,
        }
    }

    delegate_components! {
        RaiseAppErrors {
            [
                io::Error,
                serde_json::Error,
            ]:
                RaiseFrom,
        }
    }

    impl ProvideConfigType<App> for AppComponents {
        type Config = AppConfig;
    }

    impl ConfigPathGetter<App> for AppComponents {
        fn config_path(app: &App) -> &PathBuf {
            &app.config_path
        }
    }

    pub trait CanUseApp: CanLoadConfig {}

    impl CanUseApp for App {}
}
# }
```

The `App` context has a `config_path` field to store the path to the JSON config.
We also define an example `AppConfig` type, which implements `Deserialize` and has
an `api_secret` string field that can be used by further implementation.

Inside the component wiring for `AppComponents`, we make use of `UseAnyhowError`
that we have defined in earlier chapter to provide the `anyhow::Error` type,
and we use `UseDelegate<RaiseAppErrors>` to implement the error raiser.
Inside of `RaiseAppErrors`, we make use of `RaiseFrom` to convert `std::io::Error`
and `serde_json::Error` to `anyhow::Error` using the `From` instance.

We also provide context-specific implementations of `ProvideConfigType` and
`ConfigPathGetter` for the `App` context. Following that, we define a check
trait `CanUseApp` to check that the wiring is done correctly and that `App`
implements `CanLoadConfig`.

Even though the example implementation for `LoadJsonConfig` works, we would
quickly find out that the error message returned from it is not very helpful.
For example, if the file does not exist, we would get the following error
message:

```text
No such file or directory (os error 2)
```

Similarly, if the config file is not in JSON format, we would get an error message like
the following:

```text
expected value at line 1 column 2
```

Error messages like above make it very difficult for users to figure out what went
wrong, and what action needs to be taken to resolve them. To improve the
error messages, we need to _wrap_ around source errors like `std::io::Error`,
and provide additional details so that the user knows that the error occured
when trying to load the app config.
Next, we will learn about how to wrap around these errors in CGP.

## Error Wrapper

With the same motivation described in the [previous chapter](./error-reporting.md),
we would like to make use of CGP to also enable modular error reporting for the
error details that is being wrapped. This would mean that we want to define a
generic `Detail` type that can include _structured data_ inside the error
details. We can do that by introduce an _error wrapper_ trait as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_component {
    provider: ErrorWrapper,
}]
pub trait CanWrapError<Detail>: HasErrorType {
    fn wrap_error(error: Self::Error, detail: Detail) -> Self::Error;
}
```

The `CanWrapError` trait is parameterized by a generic `Detail` type, and has `HasErrorType`
as its supertrait. Inside the `wrap_error` method, it first accepts a context error `Self::Error`
and also a `Detail` value. It then wraps the detail inside the context error, and return
`Self::Error`.

To see how `CanWrapError` works in practice, we can redefine `LoadJsonConfig` to use
`CanWrapError` as follows:

```rust
# extern crate cgp;
# extern crate serde;
# extern crate serde_json;
#
# use std::path::PathBuf;
# use core::fmt::Display;
# use std::{fs, io};
#
# use cgp::prelude::*;
# use serde::Deserialize;
#
# #[cgp_component {
#     name: ConfigTypeComponent,
#     provider: ProvideConfigType,
# }]
# pub trait HasConfigType {
#     type Config;
# }
#
# #[cgp_component {
#     provider: ConfigLoader,
# }]
# pub trait CanLoadConfig: HasConfigType + HasErrorType {
#     fn load_config(&self) -> Result<Self::Config, Self::Error>;
# }
#
# #[cgp_component {
#     provider: ConfigPathGetter,
# }]
# pub trait HasConfigPath {
#     fn config_path(&self) -> &PathBuf;
# }
#
pub struct LoadJsonConfig;

impl<Context> ConfigLoader<Context> for LoadJsonConfig
where
    Context: HasConfigType
        + HasConfigPath
        + CanWrapError<String>
        + CanRaiseError<io::Error>
        + CanRaiseError<serde_json::Error>,
    Context::Config: for<'a> Deserialize<'a>,
{
    fn load_config(context: &Context) -> Result<Context::Config, Context::Error> {
        let config_path = context.config_path();

        let config_bytes = fs::read(config_path).map_err(|e| {
            Context::wrap_error(
                Context::raise_error(e),
                format!(
                    "error when reading config file at path {}",
                    config_path.display()
                ),
            )
        })?;

        let config = serde_json::from_slice(&config_bytes).map_err(|e| {
            Context::wrap_error(
                Context::raise_error(e),
                format!(
                    "error when parsing JSON config file at path {}",
                    config_path.display()
                ),
            )
        })?;

        Ok(config)
    }
}
```

Inside the new implementation of `LoadJsonConfig`, we add a `CanWrapError<String>` constraint
so that we can add stringly error details inside the provider.
When mapping the errors returned from `std::fs::read` and `serde_json::from_slice`,
we pass in a closure instead of directly calling `Context::raise_error`.
Since the first argument of `wrap_error` expects a `Context::Error`, we would
still first use `Context::raise_error` to raise `std::io::Error` and `serde_json::Error`
into `Context::Error`.
In the second argument, we use `format!` to add additional details that the errors
occured when we are trying to read and parse the given config file.

By looking only at the example, it may seem redundant that we have to first raise
a concrete source error like `std::io::Error` into `Context::Error`, before
wrapping it again using `Context::wrap_error`. If the reader prefers, you can
also use a constraint like `CanRaiseError<(String, std::io::Error)>` to raise
the I/O error with additional string detail.

However, the interface for `CanWrapError` is more applicable generally, especially
when we combine the use with other abstractions. For example, we may want to define
a trait like `CanReadFile` to try reading a file, and returning a general `Context::Error`
when the read fails. In that case, we can still use `wrap_error` without knowing
about whether we are dealing with concrete errors or abstract errors.

Next, we would need to implement a provider for `CanWrapError` to handle how to
wrap additional details into the error value. In the case when the context error
type is `anyhow::Error`, we can simply call the `context` method.
So we can implement an error wrapper provider for `anyhow::Error` as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
# use cgp::core::error::ErrorWrapper;
#
pub struct WrapWithAnyhowContext;

impl<Context, Detail> ErrorWrapper<Context, Detail> for WrapWithAnyhowContext
where
    Context: HasErrorType<Error = anyhow::Error>,
    Detail: Display + Send + Sync + 'static,
{
    fn wrap_error(error: anyhow::Error, detail: Detail) -> anyhow::Error {
        error.context(detail)
    }
}
```

We implement `WrapWithAnyhowContext` as a context-generic provider for `anyhow::Error`.
It is implemented for any context type `Context` with `Context::Error` being the same as
`anyhow::Error`. Additionally, it is implemented for any `Detail` type that implements
`Display + Send + Sync + 'static`, as those are the required trait bounds to use
`anyhow::Error::context`.
Inside the `wrap_error` implementation, we simply call `error.context(detail)` to
wrap the error detail using `anyhow`.

After rewiring the application with the new providers, if we run the application again
with missing file, it would show the following error instead:

```text
error when reading config file at path config.json

Caused by:
    No such file or directory (os error 2)
```

Similarly, when encountering error parsing the config JSON, the application now shows
the error message:

```text
error when parsing JSON config file at path config.toml

Caused by:
    expected value at line 1 column 2
```

As we can see, the error messages are now much more informative, allowing the user to diagnose
what went wrong and fix the problem.

## Structured Error Wrapping

Similar to the reasons for using structured error reporting from the
[previous chapter](./error-reporting.md), using structured error details would make it
possible to decouple how to format the wrapped error detail from the provider.
For the case of `LoadJsonConfig`, we can define and use a structured error detail
type as follows:

```rust
# extern crate cgp;
# extern crate serde;
# extern crate serde_json;
#
# use std::path::PathBuf;
# use core::fmt::Debug;
# use std::{fs, io};
#
# use cgp::prelude::*;
# use serde::Deserialize;
#
# #[cgp_component {
#     name: ConfigTypeComponent,
#     provider: ProvideConfigType,
# }]
# pub trait HasConfigType {
#     type Config;
# }
#
# #[cgp_component {
#     provider: ConfigLoader,
# }]
# pub trait CanLoadConfig: HasConfigType + HasErrorType {
#     fn load_config(&self) -> Result<Self::Config, Self::Error>;
# }
#
# #[cgp_component {
#     provider: ConfigPathGetter,
# }]
# pub trait HasConfigPath {
#     fn config_path(&self) -> &PathBuf;
# }
#
pub struct LoadJsonConfig;

pub struct ErrLoadJsonConfig<'a, Context> {
    pub context: &'a Context,
    pub config_path: &'a PathBuf,
    pub action: LoadJsonConfigAction,
}

pub enum LoadJsonConfigAction {
    ReadFile,
    ParseFile,
}

impl<Context> ConfigLoader<Context> for LoadJsonConfig
where
    Context: HasConfigType
        + HasConfigPath
        + CanRaiseError<io::Error>
        + CanRaiseError<serde_json::Error>
        + for<'a> CanWrapError<ErrLoadJsonConfig<'a, Context>>,
    Context::Config: for<'a> Deserialize<'a>,
{
    fn load_config(context: &Context) -> Result<Context::Config, Context::Error> {
        let config_path = context.config_path();

        let config_bytes = fs::read(config_path).map_err(|e| {
            Context::wrap_error(
                Context::raise_error(e),
                ErrLoadJsonConfig {
                    context,
                    config_path,
                    action: LoadJsonConfigAction::ReadFile,
                },
            )
        })?;

        let config = serde_json::from_slice(&config_bytes).map_err(|e| {
            Context::wrap_error(
                Context::raise_error(e),
                ErrLoadJsonConfig {
                    context,
                    config_path,
                    action: LoadJsonConfigAction::ParseFile,
                },
            )
        })?;

        Ok(config)
    }
}

impl<'a, Context> Debug for ErrLoadJsonConfig<'a, Context> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.action {
            LoadJsonConfigAction::ReadFile => {
                write!(
                    f,
                    "error when reading config file at path {}",
                    self.config_path.display()
                )
            }
            LoadJsonConfigAction::ParseFile => {
                write!(
                    f,
                    "error when parsing JSON config file at path {}",
                    self.config_path.display()
                )
            }
        }
    }
}
```

We first define an error detail struct `ErrLoadJsonConfig` that is parameterized
by a lifetime `'a` and a context type `Context`. Inside the struct, we include
the `Context` field to allow potential extra details to be included from the
concrete context. We also include the `config_path` to show the path of the
config file that cause the error. Lastly, we also include a `LoadJsonConfigAction`
field to indicate whether the error happened when reading or parsing the config file.

We also implement a `Debug` instance for `ErrLoadJsonConfig`, so that it can be
used by default when there is no need to customize the display of the error detail.
The `Debug` implementation ignores the `context` field, and shows the same
error messages as we did before.

To make use of the `Debug` implementation with `anyhow`, we can implement
a separate provider that wraps any `Detail` type that implements `Debug`
as follows:

```rust
# extern crate cgp;
# extern crate anyhow;
#
# use core::fmt::Debug;
#
# use cgp::prelude::*;
# use cgp::core::error::ErrorWrapper;
#
pub struct WrapWithAnyhowDebug;

impl<Context, Detail> ErrorWrapper<Context, Detail> for WrapWithAnyhowDebug
where
    Context: HasErrorType<Error = anyhow::Error>,
    Detail: Debug,
{
    fn wrap_error(error: anyhow::Error, detail: Detail) -> anyhow::Error {
        error.context(format!("{detail:?}"))
    }
}
```

To wrap the error, we first use `Debug` to format the error detail into string,
and then call `error.context` with the string.

## Full Example

With everything that we have learned so far, we can rewrite the config loader
example in the beginning of this chapter, and make use of `CanWrapError` to
decouple the error wrapping details from the provider `LoadJsonConfig`:

```rust
# extern crate anyhow;
# extern crate cgp;
# extern crate serde;
# extern crate serde_json;
#
# pub mod main {
pub mod traits {
    use std::path::PathBuf;

    use cgp::core::component::UseDelegate;
    use cgp::prelude::*;

    #[cgp_component {
        name: ConfigTypeComponent,
        provider: ProvideConfigType,
    }]
    pub trait HasConfigType {
        type Config;
    }

    #[cgp_component {
        provider: ConfigLoader,
    }]
    pub trait CanLoadConfig: HasConfigType + HasErrorType {
        fn load_config(&self) -> Result<Self::Config, Self::Error>;
    }

    #[cgp_component {
        provider: ConfigPathGetter,
    }]
    pub trait HasConfigPath {
        fn config_path(&self) -> &PathBuf;
    }
}

pub mod impls {
    use core::fmt::{Debug, Display};
    use std::path::PathBuf;
    use std::{fs, io};

    use cgp::core::error::{ErrorRaiser, ErrorWrapper,ProvideErrorType};
    use cgp::prelude::*;
    use serde::Deserialize;

    use super::traits::*;

    pub struct LoadJsonConfig;

    pub struct ErrLoadJsonConfig<'a, Context> {
        pub context: &'a Context,
        pub config_path: &'a PathBuf,
        pub action: LoadJsonConfigAction,
    }

    pub enum LoadJsonConfigAction {
        ReadFile,
        ParseFile,
    }

    impl<Context> ConfigLoader<Context> for LoadJsonConfig
    where
        Context: HasConfigType
            + HasConfigPath
            + CanRaiseError<io::Error>
            + CanRaiseError<serde_json::Error>
            + for<'a> CanWrapError<ErrLoadJsonConfig<'a, Context>>,
        Context::Config: for<'a> Deserialize<'a>,
    {
        fn load_config(context: &Context) -> Result<Context::Config, Context::Error> {
            let config_path = context.config_path();

            let config_bytes = fs::read(config_path).map_err(|e| {
                Context::wrap_error(
                    Context::raise_error(e),
                    ErrLoadJsonConfig {
                        context,
                        config_path,
                        action: LoadJsonConfigAction::ReadFile,
                    },
                )
            })?;

            let config = serde_json::from_slice(&config_bytes).map_err(|e| {
                Context::wrap_error(
                    Context::raise_error(e),
                    ErrLoadJsonConfig {
                        context,
                        config_path,
                        action: LoadJsonConfigAction::ParseFile,
                    },
                )
            })?;

            Ok(config)
        }
    }

    impl<'a, Context> Debug for ErrLoadJsonConfig<'a, Context> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self.action {
                LoadJsonConfigAction::ReadFile => {
                    write!(
                        f,
                        "error when reading config file at path {}",
                        self.config_path.display()
                    )
                }
                LoadJsonConfigAction::ParseFile => {
                    write!(
                        f,
                        "error when parsing JSON config file at path {}",
                        self.config_path.display()
                    )
                }
            }
        }
    }

    pub struct UseAnyhowError;

    impl<Context> ProvideErrorType<Context> for UseAnyhowError {
        type Error = anyhow::Error;
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

    pub struct WrapWithAnyhowDebug;

    impl<Context, Detail> ErrorWrapper<Context, Detail> for WrapWithAnyhowDebug
    where
        Context: HasErrorType<Error = anyhow::Error>,
        Detail: Debug,
    {
        fn wrap_error(error: anyhow::Error, detail: Detail) -> anyhow::Error {
            error.context(format!("{detail:?}"))
        }
    }
}

pub mod contexts {
    use std::io;
    use std::path::PathBuf;

    use cgp::core::component::UseDelegate;
    use cgp::core::error::{ErrorRaiserComponent, ErrorWrapperComponent, ErrorTypeComponent};
    use cgp::prelude::*;
    use serde::Deserialize;

    use super::impls::*;
    use super::traits::*;

    pub struct App {
        pub config_path: PathBuf,
    }

    #[derive(Deserialize)]
    pub struct AppConfig {
        pub secret: String,
    }

    pub struct AppComponents;

    pub struct RaiseAppErrors;

    impl HasComponents for App {
        type Components = AppComponents;
    }

    delegate_components! {
        AppComponents {
            ErrorTypeComponent: UseAnyhowError,
            ErrorRaiserComponent: UseDelegate<RaiseAppErrors>,
            ErrorWrapperComponent: WrapWithAnyhowDebug,
            ConfigLoaderComponent: LoadJsonConfig,
        }
    }

    delegate_components! {
        RaiseAppErrors {
            [
                io::Error,
                serde_json::Error,
            ]:
                RaiseFrom,
        }
    }

    impl ProvideConfigType<App> for AppComponents {
        type Config = AppConfig;
    }

    impl ConfigPathGetter<App> for AppComponents {
        fn config_path(app: &App) -> &PathBuf {
            &app.config_path
        }
    }

    pub trait CanUseApp: CanLoadConfig {}

    impl CanUseApp for App {}
}
# }
```

## Delegated Error Wrapping

Similar to the previous chapter on [delegated error raisers](./delegated-error-raiser.md),
we can also make use of the `UseDelegate` pattern to implement delegated error wrapping as follows:


```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
# use cgp::core::error::ErrorWrapper;
#
# pub struct UseDelegate<Components>(pub PhantomData<Components>);
#
impl<Context, Detail, Components> ErrorWrapper<Context, Detail> for UseDelegate<Components>
where
    Context: HasErrorType,
    Components: DelegateComponent<Detail>,
    Components::Delegate: ErrorWrapper<Context, Detail>,
{
    fn wrap_error(error: Context::Error, detail: Detail) -> Context::Error {
        Components::Delegate::wrap_error(error, detail)
    }
}
```

With this implementation, we can dispatch the handling of different error `Detail` type
to different error wrappers, similar to how we dispatch the error raisers based on the
`SourceError` type:

```rust
# extern crate anyhow;
# extern crate cgp;
# extern crate serde;
# extern crate serde_json;
#
# pub mod main {
# pub mod traits {
#     use std::path::PathBuf;
#
#     use cgp::core::component::UseDelegate;
#     use cgp::core::error::ErrorWrapper;
#     use cgp::prelude::*;
#
#     #[cgp_component {
#         name: ConfigTypeComponent,
#         provider: ProvideConfigType,
#     }]
#     pub trait HasConfigType {
#         type Config;
#     }
#
#     #[cgp_component {
#         provider: ConfigLoader,
#     }]
#     pub trait CanLoadConfig: HasConfigType + HasErrorType {
#         fn load_config(&self) -> Result<Self::Config, Self::Error>;
#     }
#
#     #[cgp_component {
#         provider: ConfigPathGetter,
#     }]
#     pub trait HasConfigPath {
#         fn config_path(&self) -> &PathBuf;
#     }
# }
#
# pub mod impls {
#     use core::fmt::{Debug, Display};
#     use std::path::PathBuf;
#     use std::{fs, io};
#
#     use cgp::core::error::{ErrorRaiser, ErrorWrapper, ProvideErrorType};
#     use cgp::prelude::*;
#     use serde::Deserialize;
#
#     use super::traits::*;
#
#     pub struct LoadJsonConfig;
#
#     pub struct ErrLoadJsonConfig<'a, Context> {
#         pub context: &'a Context,
#         pub config_path: &'a PathBuf,
#         pub action: LoadJsonConfigAction,
#     }
#
#     pub enum LoadJsonConfigAction {
#         ReadFile,
#         ParseFile,
#     }
#
#     impl<Context> ConfigLoader<Context> for LoadJsonConfig
#     where
#         Context: HasConfigType
#             + HasConfigPath
#             + CanRaiseError<io::Error>
#             + CanRaiseError<serde_json::Error>
#             + for<'a> CanWrapError<ErrLoadJsonConfig<'a, Context>>,
#         Context::Config: for<'a> Deserialize<'a>,
#     {
#         fn load_config(context: &Context) -> Result<Context::Config, Context::Error> {
#             let config_path = context.config_path();
#
#             let config_bytes = fs::read(config_path).map_err(|e| {
#                 Context::wrap_error(
#                     Context::raise_error(e),
#                     ErrLoadJsonConfig {
#                         context,
#                         config_path,
#                         action: LoadJsonConfigAction::ReadFile,
#                     },
#                 )
#             })?;
#
#             let config = serde_json::from_slice(&config_bytes).map_err(|e| {
#                 Context::wrap_error(
#                     Context::raise_error(e),
#                     ErrLoadJsonConfig {
#                         context,
#                         config_path,
#                         action: LoadJsonConfigAction::ParseFile,
#                     },
#                 )
#             })?;
#
#             Ok(config)
#         }
#     }
#
#     impl<'a, Context> Debug for ErrLoadJsonConfig<'a, Context> {
#         fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
#             match self.action {
#                 LoadJsonConfigAction::ReadFile => {
#                     write!(
#                         f,
#                         "error when reading config file at path {}",
#                         self.config_path.display()
#                     )
#                 }
#                 LoadJsonConfigAction::ParseFile => {
#                     write!(
#                         f,
#                         "error when parsing JSON config file at path {}",
#                         self.config_path.display()
#                     )
#                 }
#             }
#         }
#     }
#
#     pub struct UseAnyhowError;
#
#     impl<Context> ProvideErrorType<Context> for UseAnyhowError {
#         type Error = anyhow::Error;
#     }
#
#     pub struct RaiseFrom;
#
#     impl<Context, SourceError> ErrorRaiser<Context, SourceError> for RaiseFrom
#     where
#         Context: HasErrorType,
#         Context::Error: From<SourceError>,
#     {
#         fn raise_error(e: SourceError) -> Context::Error {
#             e.into()
#         }
#     }
#
#     pub struct WrapWithAnyhowContext;
#
#     impl<Context, Detail> ErrorWrapper<Context, Detail> for WrapWithAnyhowContext
#     where
#         Context: HasErrorType<Error = anyhow::Error>,
#         Detail: Display + Send + Sync + 'static,
#     {
#         fn wrap_error(error: anyhow::Error, detail: Detail) -> anyhow::Error {
#             error.context(detail)
#         }
#     }
#
#     pub struct WrapWithAnyhowDebug;
#
#     impl<Context, Detail> ErrorWrapper<Context, Detail> for WrapWithAnyhowDebug
#     where
#         Context: HasErrorType<Error = anyhow::Error>,
#         Detail: Debug,
#     {
#         fn wrap_error(error: anyhow::Error, detail: Detail) -> anyhow::Error {
#             error.context(format!("{detail:?}"))
#         }
#     }
# }
#
# pub mod contexts {
#     use std::io;
#     use std::path::PathBuf;
#
#     use cgp::core::component::UseDelegate;
#     use cgp::core::error::{ErrorRaiserComponent, ErrorWrapperComponent, ErrorTypeComponent};
#     use cgp::prelude::*;
#     use serde::Deserialize;
#
#     use super::impls::*;
#     use super::traits::*;
#
pub struct App {
    pub config_path: PathBuf,
}

#[derive(Deserialize)]
pub struct AppConfig {
    pub secret: String,
}

pub struct AppComponents;

pub struct RaiseAppErrors;

pub struct WrapAppErrors;

impl HasComponents for App {
    type Components = AppComponents;
}

delegate_components! {
    AppComponents {
        ErrorTypeComponent: UseAnyhowError,
        ErrorRaiserComponent: UseDelegate<RaiseAppErrors>,
        ErrorWrapperComponent: UseDelegate<WrapAppErrors>,
        ConfigLoaderComponent: LoadJsonConfig,
    }
}

delegate_components! {
    RaiseAppErrors {
        [
            io::Error,
            serde_json::Error,
        ]:
            RaiseFrom,
    }
}

delegate_components! {
    WrapAppErrors {
        String: WrapWithAnyhowContext,
        <'a, Context> ErrLoadJsonConfig<'a, Context>:
            WrapWithAnyhowDebug,
        // add other error wrappers here
    }
}
#
#     impl ProvideConfigType<App> for AppComponents {
#         type Config = AppConfig;
#     }
#
#     impl ConfigPathGetter<App> for AppComponents {
#         fn config_path(app: &App) -> &PathBuf {
#             &app.config_path
#         }
#     }
#
#     pub trait CanUseApp: CanLoadConfig {}
#
#     impl CanUseApp for App {}
# }
# }
```

The above example shows the addition of a new `WrapAppErrors` type, which we
use with `delegate_components!` to map the handling of
`String` detail to `WrapWithAnyhowContext`, and `ErrLoadJsonConfig` detail to
`WrapWithAnyhowDebug`. Following the same pattern, we will be able to customize
how exactly each error detail is wrapped, by updating the mapping for `WrapAppErrors`.

## Conclusion

In this chapter, we learned about how to perform abstract error wrapping to wrap additional
details to an abstract error. The pattern for using `CanWrapError` is very similar to the
patterns that we have previously learned for `CanRaiseError`. So this is mostly a recap
of the same patterns, and also show readers how you can expect the same CGP pattern
to be applied in many different places.

Similar to the advice from the previous chapters, it could be overwhelming for beginners
to try to use the full structured error wrapping patterns introduced in this chapter.
As a result, we encourage readers to start with using only `String` as the error detail
when wrapping errors inside practice applications.

The need for structured error wrapping typically would only arise in large-scale applications,
or when one wants to publish CGP-based library crates for others to build modular applications.
As such, you can always revisit this chapter at a later time, and refactor your providers
to make use of structured error details when you really need them.