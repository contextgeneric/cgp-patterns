pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    #[cgp_component {
        name: TimeTypeComponent,
        provider: ProvideTimeType,
    }]
    pub trait HasTimeType {
        type Time;
    }

    #[cgp_component {
        name: AuthTokenTypeComponent,
        provider: ProvideAuthTokenType,
    }]
    pub trait HasAuthTokenType {
        type AuthToken;
    }

    #[cgp_component {
        provider: AuthTokenValidator,
    }]
    pub trait CanValidateAuthToken: HasAuthTokenType {
        fn validate_auth_token(&self, auth_token: &Self::AuthToken) -> Result<(), Error>;
    }

    #[cgp_component {
        provider: AuthTokenExpiryFetcher,
    }]
    pub trait CanFetchAuthTokenExpiry: HasAuthTokenType + HasTimeType {
        fn fetch_auth_token_expiry(
            &self,
            auth_token: &Self::AuthToken,
        ) -> Result<Self::Time, Error>;
    }

    #[cgp_component {
        provider: CurrentTimeGetter,
    }]
    pub trait HasCurrentTime: HasTimeType {
        fn current_time(&self) -> Result<Self::Time, Error>;
    }
}

pub mod impls {
    use anyhow::{anyhow, Error};
    use datetime::LocalDateTime;

    use super::traits::*;

    pub struct ValidateTokenIsNotExpired;

    impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
    where
        Context: HasCurrentTime + CanFetchAuthTokenExpiry,
        Context::Time: Ord,
    {
        fn validate_auth_token(
            context: &Context,
            auth_token: &Context::AuthToken,
        ) -> Result<(), Error> {
            let now = context.current_time()?;

            let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

            if token_expiry < now {
                Ok(())
            } else {
                Err(anyhow!("auth token has expired"))
            }
        }
    }

    pub struct UseLocalDateTime;

    impl<Context> ProvideTimeType<Context> for UseLocalDateTime {
        type Time = LocalDateTime;
    }

    impl<Context> CurrentTimeGetter<Context> for UseLocalDateTime
    where
        Context: HasTimeType<Time = LocalDateTime>,
    {
        fn current_time(_context: &Context) -> Result<LocalDateTime, Error> {
            Ok(LocalDateTime::now())
        }
    }

    pub struct UseStringAuthToken;

    impl<Context> ProvideAuthTokenType<Context> for UseStringAuthToken {
        type AuthToken = String;
    }
}

pub mod contexts {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use cgp::prelude::*;
    use datetime::LocalDateTime;

    use super::impls::*;
    use super::traits::*;

    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, LocalDateTime>,
    }

    pub struct MockAppComponents;

    impl HasComponents for MockApp {
        type Components = MockAppComponents;
    }

    delegate_components! {
        MockAppComponents {
            [
                TimeTypeComponent,
                CurrentTimeGetterComponent,
            ]: UseLocalDateTime,
            AuthTokenTypeComponent: UseStringAuthToken,
            AuthTokenValidatorComponent: ValidateTokenIsNotExpired,
        }
    }

    impl AuthTokenExpiryFetcher<MockApp> for MockAppComponents {
        fn fetch_auth_token_expiry(
            context: &MockApp,
            auth_token: &String,
        ) -> Result<LocalDateTime, anyhow::Error> {
            context
                .auth_tokens_store
                .get(auth_token)
                .cloned()
                .ok_or_else(|| anyhow!("invalid auth token"))
        }
    }

    pub trait CanUseMockApp: CanValidateAuthToken {}

    impl CanUseMockApp for MockApp {}
}
