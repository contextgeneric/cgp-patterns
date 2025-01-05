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

    #[cgp_component {
        provider: ErrorWrapper,
    }]
    pub trait CanWrapError<Detail>: HasErrorType {
        fn wrap_error(error: Self::Error, detail: Detail) -> Self::Error;
    }

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
}

pub mod impls {
    use core::fmt::{Debug, Display};
    use std::path::PathBuf;
    use std::{fs, io};

    use cgp::core::error::{ErrorRaiser, ProvideErrorType};
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
            <'a, Context> ErrLoadJsonConfig<'a, Context>:
                WrapWithAnyhowDebug,
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
