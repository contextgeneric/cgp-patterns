# Field Accessors

Using impl-side dependencies, CGP provides a way to inject dependencies into providers
without polluting the public interfaces with additional constraints. A common use of
dependency injection is for the provider to retrieve some values from the context.
More commonly, we call this pattern field _accessor_ or _getter_, since we are getting
or accessing field values from the context.
In this chapter, we will walk through how to effectively define and use field accessors
with CGP.

## Example: API Call

Supposed that our application needs to make API calls to an external services to read
messages by message ID. To abstract away the details of the API call, we would define
CGP traits such as follows:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

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
```

Following the patterns for [associated types](./associated-types.md), we define the
type traits `HasMessageIdType` and `HasMessageType` to abstract away the detailed structures
of the message ID and messages.
Following the patterns for [error handling](./error-handling.md), we define the
`CanQueryMessage` trait to accept an abstract `MessageId` value, and return
either an abstract `Message` or an abstract `Error`.

With the interfaces defined, we will then try and implement a naive API client provider
that queries the message as HTTP request:

```rust
# extern crate cgp;
# extern crate reqwest;
# extern crate serde;
#
# use cgp::prelude::*;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

# #[cgp_component {
#     name: MessageIdTypeComponent,
#     provider: ProvideMessageIdType,
# }]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_component {
#     name: MessageTypeComponent,
#     provider: ProvideMessageType,
# }]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_component {
#     provider: MessageQuerier,
# }]
# pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
#     fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
# }
#
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
```

For the purpose of the examples here, we will use the [`reqwest`](https://docs.rs/reqwest)
library to make the HTTP calls. We will also use the _blocking_ version of the APIs
in this chapter, as we will only cover about doing asynchronous programming in CGP in
later chapters.

In the above example, we implement `MessageQuerier` for the provider `ReadMessageFromApi`.
For simplicity, we require the additional constraint that `MessageId` needs to be
`u64`, and the `Message` type is just a simple `String`.
We also make use of the context to raise the `reqwest::Error` returned from calling
`reqwest` methods, and also a custom `ErrStatusCode` error in case if the server
returns error HTTP response.

Inside the method body, we first build a reqwest `Client`, and then use it to issue
a HTTP GET request to the URL `"http://localhost:8000/api/messages/{message_id}"`.
If the returned HTTP status is not successful, we raise the error `ErrStatusCode`.
Otherwise, we parse the response body as JSON using the `ApiMessageResponse` struct,
which expects the response body to contain a `message` string field.

We may quickly notice that the naive provider has several things hard coded.
For start, it has the hardcoded API base URL `http://localhost:8000`, which should
be made configurable. We will next walk through how to define _accessor_ traits
to access these configurable values from the context.

## Getting the Base API URL

Using CGP, it is pretty straightforward to define an accessor trait for getting
values from the context. To make the base API URL configurable, we would define
a `HasApiBaseUrl` trait as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_component {
    provider: ApiBaseUrlGetter,
}]
pub trait HasApiBaseUrl {
    fn api_base_url(&self) -> &String;
}
```

The trait `HasApiBaseUrl` provides a method `api_base_url`, which returns a `&String`
from the context. In production applications, we may want the method to return a
[`Url`](https://docs.rs/url/latest/url/struct.Url.html), or even an abstract `Url` type.
But we will use strings here to keep the example simple.

We can then include `HasApiBaseUrl` inside `ReadMessageFromApi`, so that we can
construct the HTTP request using the base API URL provided by the context:

```rust
# extern crate cgp;
# extern crate reqwest;
# extern crate serde;
#
# use cgp::prelude::*;
# use reqwest::blocking::Client;
# use reqwest::StatusCode;
# use serde::Deserialize;
#
# #[cgp_component {
#     name: MessageIdTypeComponent,
#     provider: ProvideMessageIdType,
# }]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_component {
#     name: MessageTypeComponent,
#     provider: ProvideMessageType,
# }]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_component {
#     provider: MessageQuerier,
# }]
# pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
#     fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
# }
#
# #[cgp_component {
#     provider: ApiBaseUrlGetter,
# }]
# pub trait HasApiBaseUrl {
#     fn api_base_url(&self) -> &String;
# }
#
# pub struct ReadMessageFromApi;
#
# #[derive(Debug)]
# pub struct ErrStatusCode {
#     pub status_code: StatusCode,
# }
#
# #[derive(Deserialize)]
# pub struct ApiMessageResponse {
#     pub message: String,
# }
#
impl<Context> MessageQuerier<Context> for ReadMessageFromApi
where
    Context: HasMessageIdType<MessageId = u64>
        + HasMessageType<Message = String>
        + HasApiBaseUrl
        + CanRaiseError<reqwest::Error>
        + CanRaiseError<ErrStatusCode>,
{
    fn query_message(context: &Context, message_id: &u64) -> Result<String, Context::Error> {
        let client = Client::new();

        let url = format!("{}/api/messages/{}", context.api_base_url(), message_id);

        let response = client.get(url).send().map_err(Context::raise_error)?;

        let status_code = response.status();

        if !status_code.is_success() {
            return Err(Context::raise_error(ErrStatusCode { status_code }));
        }

        let message_response: ApiMessageResponse = response.json().map_err(Context::raise_error)?;

        Ok(message_response.message)
    }
}
```

## Getting the Auth Token

Aside from the base API URL, it is common for API services to require some kind of authentication
to protect the API resource from being accessed by unauthorized party.
For the purpose of this example, we will make use of simple _bearer tokens_ to access the API.

Similar to `HasApiBaseUrl`, we will define a `HasAuthToken` getter to get the
auth token as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_component {
    name: AuthTokenTypeComponent,
    provider: ProvideAuthTokenType,
}]
pub trait HasAuthTokenType {
    type AuthToken;
}

#[cgp_component {
    provider: AuthTokenGettter,
}]
pub trait HasAuthToken: HasAuthTokenType {
    fn auth_token(&self) -> &Self::AuthToken;
}
```

Similar to the [earlier chapter](./associated-types.md), we first define `HasAuthTokenType`
to keep the `AuthToken` type abstract. In fact, the same `HasAuthTokenType` trait
and their respective providers could be reused across the chapters. This also shows
that having minimal CGP traits make it easier to reuse the same interface across
different applications.

We then define a getter trait `HasAuthToken`, to get an abstract `AuthToken` value from
the context. We can then update `ReadMessageFromApi` to include the auth token
inside the `Authorization` HTTP header:

```rust
# extern crate cgp;
# extern crate reqwest;
# extern crate serde;
#
# use core::fmt::Display;
#
# use cgp::prelude::*;
# use reqwest::blocking::Client;
# use reqwest::StatusCode;
# use serde::Deserialize;
#
# #[cgp_component {
#     name: MessageIdTypeComponent,
#     provider: ProvideMessageIdType,
# }]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_component {
#     name: MessageTypeComponent,
#     provider: ProvideMessageType,
# }]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_component {
#     provider: MessageQuerier,
# }]
# pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
#     fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
# }
#
# #[cgp_component {
#     provider: ApiBaseUrlGetter,
# }]
# pub trait HasApiBaseUrl {
#     fn api_base_url(&self) -> &String;
# }
#
# #[cgp_component {
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component {
#     provider: AuthTokenGettter,
# }]
# pub trait HasAuthToken: HasAuthTokenType {
#     fn auth_token(&self) -> &Self::AuthToken;
# }
#
# pub struct ReadMessageFromApi;
#
# #[derive(Debug)]
# pub struct ErrStatusCode {
#     pub status_code: StatusCode,
# }
#
# #[derive(Deserialize)]
# pub struct ApiMessageResponse {
#     pub message: String,
# }
#
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
```

In the updated code, we make use of reqwest's
[`bearer_auth`](https://docs.rs/reqwest/latest/reqwest/blocking/struct.RequestBuilder.html#method.bearer_auth)
method to include the auth token into the HTTP header.
In this case, the provider only require `Context::AuthToken` to implement `Display`,
making it possible to be used with custom `AuthToken` types other than `String`.