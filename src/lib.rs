pub mod types {
    pub struct WrapError<Detail, Error> {
        pub detail: Detail,
        pub error: Error,
    }
}

pub mod traits {
    use std::path::PathBuf;

    use cgp::prelude::*;

    use super::types::*;

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
}

pub mod impls {
    use core::fmt::Display;
    use std::{fs, io};

    use cgp::core::error::{ErrorRaiser, ProvideErrorType};
    use cgp::prelude::*;
    use serde::Deserialize;

    use super::traits::*;
    use super::types::*;

    pub struct LoadConfigJson;

    impl<Context> ConfigLoader<Context> for LoadConfigJson
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
                    format!(
                        "error when reading config file at path {}",
                        config_path.display()
                    ),
                    Context::raise_error(e),
                )
            })?;

            let config = serde_json::from_slice(&config_bytes).map_err(|e| {
                Context::wrap_error(
                    format!(
                        "error when parsing config file at path {}",
                        config_path.display()
                    ),
                    Context::raise_error(e),
                )
            })?;

            Ok(config)
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

    pub struct WrapWithAnyhow;

    impl<Context, Detail> ErrorRaiser<Context, WrapError<Detail, anyhow::Error>> for WrapWithAnyhow
    where
        Context: HasErrorType<Error = anyhow::Error>,
        Detail: Display + Send + Sync + 'static,
    {
        fn raise_error(e: WrapError<Detail, anyhow::Error>) -> anyhow::Error {
            e.error.context(e.detail)
        }
    }
}

pub mod contexts {
    use std::io;
    use std::path::PathBuf;

    use cgp::core::component::UseDelegate;
    use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
    use cgp::prelude::*;
    use serde::Deserialize;

    use super::impls::*;
    use super::traits::*;
    use super::types::*;

    pub struct App {
        pub config_path: PathBuf,
    }

    #[derive(Deserialize)]
    pub struct AppConfig {
        pub secret: String,
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
            WrapError<String, anyhow::Error>:
                WrapWithAnyhow,
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
