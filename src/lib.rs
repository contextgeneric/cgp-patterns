use cgp::prelude::*;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

#[cgp_component {
    name: MessageIdTypeComponent,
    provider: ProvideMessageIdType,
}]
pub trait HasMessageIdType {
    type MessageId;
}

#[cgp_component {
    name: MessageTypeComponent,
    provider: ProvideMessageType,
}]
pub trait HasMessageType {
    type Message;
}

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
        + CanRaiseError<reqwest::Error>
        + CanRaiseError<ErrStatusCode>,
{
    fn query_message(_context: &Context, message_id: &u64) -> Result<String, Context::Error> {
        let client = Client::new();

        let response = client
            .get(format!("http://localhost:8000/api/messages/{message_id}"))
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
