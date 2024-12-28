# Associated Types

In the first part of this book, we have learned about how CGP makes use of
Rust's trait system to wire up components using blanket implementations.
Since CGP works within Rust's trait system, we can make use of advanced
Rust features together with CGP to form new design patterns.
In this chapter, we will learn about how to make use of _associated types_
with CGP to define context-generic providers that are generic over multiple
types.

# Building Authentication Components

Supposed that we want to build a simple authentication system using _bearer tokens_
with expiry time. To build such system, we would need to fetch the expiry time of
a valid token, and ensure that the time is not in the past. A naive attempt of
implementing the authentication would be as follows:

```rust
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

            if token_expiry < now {
                Ok(())
            } else {
                Err(anyhow!("auth token has expired"))
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
```

We first define `CanValidateAuthToken`, which would be used as the main API for validating an
auth token. In order to help implementing the validator, we also define
`CanFetchAuthTokenExpiry` used for fetching the expiry time of an auth token, if it is valid.
Finally, we also define `HasCurrentTime` which is used for fetching the current time.

We then define a context-generic provider `ValidateTokenIsNotExpired`, which validates auth tokens
by fetching the token's expiry time and the current time, and ensure that the token's expiry time
does not exceed the current time. We also define a context-generic provider `GetSystemTimestamp`,
which gets the current time using `std::time::System::now()`.

For the purpose of this demo, we also define a concrete context `MockApp`, which contains a
`auth_tokens_store` field with mocked collection of auth tokens with respective expiry time
stored in a `BTreeMap`.
We then implement a context-specific provider of `AuthTokenExpiryFetcher` for `MockApp`,
which reads from the mocked `auth_tokens_store`.
We also define a check trait `CanUseMockApp`, to check that `MockApp` correctly implements
`CanValidateAuthToken` with the wiring provided.

## Abstracting Types

The naive example above makes use of basic CGP techniques to implement a reusable
`ValidateTokenIsNotExpired`, which can be used with different concrete contexts.
However, we can see that the method signatures are tied to specific types.
In particular, we used `String` to represent the auth token, and `u64` to
represent the unix timestamp in milliseconds.

Common wisdom tells us that we should use distinct types to distinguish values
from specific domains, so that we do not accidentally mix up values from different
domains. A common approach in Rust is to make use of the _newtype pattern_ to
define wrapper types such as follows:

```rust
pub struct AuthToken {
    value: String,
}

pub struct Time {
    value: u64,
}
```