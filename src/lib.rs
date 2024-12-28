pub mod traits {
    use anyhow::Error;
    use cgp::prelude::*;

    #[cgp_component {
        provider: AuthTokenValidator,
    }]
    pub trait CanValidateAuthToken {
        fn validate_auth_token(&self, auth_token: &str) -> Result<(), Error>;
    }

    #[cgp_component {
        provider: AuthTokenExpiryFetcher,
    }]
    pub trait CanFetchAuthTokenExpiry {
        fn fetch_auth_token_expiry(&self, auth_token: &str) -> Result<u64, Error>;
    }

    #[cgp_component {
        provider: CurrentTimeGetter,
    }]
    pub trait HasCurrentTime {
        fn current_time(&self) -> Result<u64, Error>;
    }
}

pub mod impls {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::{anyhow, Error};

    use super::traits::*;

    pub struct ValidateTokenIsNotExpired;

    impl<Context> AuthTokenValidator<Context> for ValidateTokenIsNotExpired
    where
        Context: HasCurrentTime + CanFetchAuthTokenExpiry,
    {
        fn validate_auth_token(context: &Context, auth_token: &str) -> Result<(), Error> {
            let now = context.current_time()?;

            let token_expiry = context.fetch_auth_token_expiry(auth_token)?;

            if token_expiry <= now {
                Err(anyhow!("auth token has expired"))
            } else {
                Ok(())
            }
        }
    }

    pub struct GetSystemTimestamp;

    impl<Context> CurrentTimeGetter<Context> for GetSystemTimestamp {
        fn current_time(_context: &Context) -> Result<u64, Error> {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis()
                .try_into()?;

            Ok(now)
        }
    }
}

pub mod contexts {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use cgp::prelude::*;

    use super::impls::*;
    use super::traits::*;

    pub struct MockApp {
        pub auth_tokens_store: BTreeMap<String, u64>,
    }

    pub struct MockAppComponents;

    impl HasComponents for MockApp {
        type Components = MockAppComponents;
    }

    delegate_components! {
        MockAppComponents {
            CurrentTimeGetterComponent: GetSystemTimestamp,
            AuthTokenValidatorComponent: ValidateTokenIsNotExpired,
        }
    }

    impl AuthTokenExpiryFetcher<MockApp> for MockAppComponents {
        fn fetch_auth_token_expiry(
            context: &MockApp,
            auth_token: &str,
        ) -> Result<u64, anyhow::Error> {
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
