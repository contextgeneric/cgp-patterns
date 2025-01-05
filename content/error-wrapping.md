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

    pub struct LoadConfigJson;

    impl<Context> ConfigLoader<Context> for LoadConfigJson
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

Using the config traits, we then implement `LoadConfigJson` as a context-generic
provider for `ConfigLoader`, which would read a JSON config file as bytes from the
filesystem using `std::fs::read`, and then parse the config using `serde_json`.
With CGP, `LoadConfigJson` can work with any `Config` type that implements `Deserialize`.

We can then define an example application context that makes use of `LoadConfigJson` to
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
#     pub struct LoadConfigJson;
#
#     impl<Context> ConfigLoader<Context> for LoadConfigJson
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

    pub struct HandleAppErrors;

    impl HasComponents for App {
        type Components = AppComponents;
    }

    delegate_components! {
        AppComponents {
            ErrorTypeComponent: UseAnyhowError,
            ErrorRaiserComponent: UseDelegate<HandleAppErrors>,
            ConfigLoaderComponent: LoadConfigJson,
        }
    }

    delegate_components! {
        HandleAppErrors {
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
and we use `UseDelegate<HandleAppErrors>` to implement the error raiser.
Inside of `HandleAppErrors`, we make use of `RaiseFrom` to convert `std::io::Error`
and `serde_json::Error` to `anyhow::Error` using the `From` instance.

We also provide context-specific implementations of `ProvideConfigType` and
`ConfigPathGetter` for the `App` context. Following that, we define a check
trait `CanUseApp` to check that the wiring is done correctly and that `App`
implements `CanLoadConfig`.

Even though the example implementation for `LoadConfigJson` works, we would
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

## Wrapped Source Error

With the same motivation described in the [previous chapter](./error-reporting.md),
we would like to make use of CGP to also enable modular error reporting for the
error details that is being wrapped. This would mean that we want to define a
generic `Detail` type that can include _structured data_ inside the error
details. When combined with the abstract error type, we would first define
a wrapper type `WrapError` to wrap the detail with the error:

```rust
pub struct WrapError<Detail, Error> {
    pub detail: Detail,
    pub error: Error,
}
```

The `WrapError` type is made of two public fields, with a `Detail` value and an
`Error` value.
We design `WrapError` to be used inside `CanRaiseError`, to add additional error
details to an error.

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# pub struct WrapError<Detail, Error> {
#     pub detail: Detail,
#     pub error: Error,
# }
#
pub trait CanWrapError<Detail>: HasErrorType {
    fn wrap_error(detail: Detail, error: Self::Error) -> Self::Error;
}

impl<Context, Detail, Error> CanWrapError<Detail> for Context
where
    Context: HasErrorType<Error = Error> + CanRaiseError<WrapError<Detail, Error>>,
{
    fn wrap_error(detail: Detail, error: Error) -> Error {
        Context::raise_error(WrapError { detail, error })
    }
}
```