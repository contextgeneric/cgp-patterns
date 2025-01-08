use core::fmt::Display;

use cgp::core::component::UseDelegate;
use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
use cgp::core::field::UseField;
use cgp::extra::error::RaiseFrom;
use cgp::prelude::*;
use cgp_error_anyhow::{DebugAnyhowError, UseAnyhowError};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

cgp_type!(Message);
cgp_type!(MessageId);
cgp_type!(AuthToken);

#[cgp_component {
    provider: MessageQuerier,
}]
pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
    fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
}

pub struct ReadMessageFromApi;

#[derive(Debug)]
pub struct ErrStatusCode {
    pub status_code: StatusCode,
}

#[derive(Deserialize)]
pub struct ApiMessageResponse {
    pub message: String,
}

impl<Context> MessageQuerier<Context> for ReadMessageFromApi
where
    Context: HasMessageIdType<MessageId = u64>
        + HasMessageType<Message = String>
        + HasApiBaseUrl
        + HasAuthToken
        + CanRaiseError<reqwest::Error>
        + CanRaiseError<ErrStatusCode>,
    Context::AuthToken: Display,
{
    fn query_message(context: &Context, message_id: &u64) -> Result<String, Context::Error> {
        let client = Client::new();

        let url = format!("{}/api/messages/{}", context.api_base_url(), message_id);

        let response = client
            .get(url)
            .bearer_auth(context.auth_token())
            .send()
            .map_err(Context::raise_error)?;

        let status_code = response.status();

        if !status_code.is_success() {
            return Err(Context::raise_error(ErrStatusCode { status_code }));
        }

        let message_response: ApiMessageResponse = response.json().map_err(Context::raise_error)?;

        Ok(message_response.message)
    }
}

#[cgp_getter {
    provider: ApiBaseUrlGetter,
}]
pub trait HasApiBaseUrl {
    fn api_base_url(&self) -> &String;
}

#[cgp_getter {
    provider: AuthTokenGetter,
}]
pub trait HasAuthToken: HasAuthTokenType {
    fn auth_token(&self) -> &Self::AuthToken;
}

#[derive(HasField)]
pub struct ApiClient {
    pub api_base_url: String,
    pub auth_token: String,
}

pub struct ApiClientComponents;

pub struct RaiseApiErrors;

impl HasComponents for ApiClient {
    type Components = ApiClientComponents;
}

delegate_components! {
    ApiClientComponents {
        ErrorTypeComponent: UseAnyhowError,
        ErrorRaiserComponent: UseDelegate<RaiseApiErrors>,
        MessageIdTypeComponent: UseType<u64>,
        MessageTypeComponent: UseType<String>,
        AuthTokenTypeComponent: UseType<String>,
        ApiBaseUrlGetterComponent: UseFields,
        AuthTokenGetterComponent: UseFields,
        MessageQuerierComponent: ReadMessageFromApi,
    }
}

delegate_components! {
    RaiseApiErrors {
        reqwest::Error: RaiseFrom,
        ErrStatusCode: DebugAnyhowError,
    }
}

pub trait CanUseApiClient: CanQueryMessage {}

impl CanUseApiClient for ApiClient {}
