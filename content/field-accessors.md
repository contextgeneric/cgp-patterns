# Field Accessors

With impl-side dependencies, CGP offers a way to inject dependencies into providers without cluttering the public interfaces with extra constraints. One common use of this dependency injection is for a provider to retrieve values from the context. This pattern is often referred to as a field _accessor_ or _getter_, since it involves accessing field values from the context. In this chapter, we'll explore how to define and use field accessors effectively with CGP.

## Example: API Call

Suppose our application needs to make API calls to an external service to read messages by their message ID. To abstract away the details of the API call, we can define CGP traits as follows:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

#[cgp_type]
pub trait HasMessageType {
    type Message;
}

#[cgp_type]
pub trait HasMessageIdType {
    type MessageId;
}

#[cgp_component(MessageQuerier)]
pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
    fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
}
```

Following the patterns for [associated types](./associated-types.md), we define the type traits `HasMessageIdType` and `HasMessageType` to abstract away the details of the message ID and message structures. Additionally, the `CanQueryMessage` trait accepts an abstract `MessageId` and returns either an abstract `Message` or an abstract `Error`, following the patterns for [error handling](./error-handling.md).

With the interfaces defined, we now implement a simple API client provider that queries the message via an HTTP request.

```rust
# extern crate cgp;
# extern crate reqwest;
# extern crate serde;
#
# use cgp::prelude::*;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
#
# #[cgp_type]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_type]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_component(MessageQuerier)]
# pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
#     fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
# }
#
#[derive(Debug)]
pub struct ErrStatusCode {
    pub status_code: StatusCode,
}

#[derive(Deserialize)]
pub struct ApiMessageResponse {
    pub message: String,
}

#[cgp_new_provider]
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

For the purposes of the examples in this chapter, we will use the [`reqwest`](https://docs.rs/reqwest) library to make HTTP calls. We will also use the _blocking_ version of the API in this chapter, as asynchronous programming in CGP will be covered in later chapters.

In the example above, we implement `MessageQuerier` for the `ReadMessageFromApi` provider. For simplicity, we add the constraint that `MessageId` must be of type `u64` and the `Message` type is a basic `String`.

We also use the context to handle errors. Specifically, we raise the `reqwest::Error` returned by the `reqwest` methods, as well as a custom `ErrStatusCode` error if the server responds with an error HTTP status.

Within the method, we first create a `reqwest::Client`, and then use it to send an HTTP GET request to the URL `"http://localhost:8000/api/messages/{message_id}"`. If the returned HTTP status is unsuccessful, we raise the `ErrStatusCode`. Otherwise, we parse the response body as JSON into the `ApiMessageResponse` struct, which expects the response to contain a `message` field.

It's clear that the naive provider has some hard-coded values. For instance, the API base URL `http://localhost:8000` is fixed, but it should be configurable. In the next section, we will explore how to define _accessor_ traits to retrieve these configurable values from the context.

## Getting the Base API URL

In CGP, defining an accessor trait to retrieve values from the context is straightforward. To make the base API URL configurable, we define a `HasApiBaseUrl` trait as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_component(ApiBaseUrlGetter)]
pub trait HasApiBaseUrl {
    fn api_base_url(&self) -> &String;
}
```

The `HasApiBaseUrl` trait defines a method, `api_base_url`, which returns a reference to a `String` from the context. In production applications, you might prefer to return a [`url::Url`](https://docs.rs/url/latest/url/struct.Url.html) or even an abstract `Url` type instead of a `String`. However, for simplicity, we use a `String` in this example.

Next, we can include the `HasApiBaseUrl` trait within `ReadMessageFromApi`, allowing us to construct the HTTP request using the base API URL provided by the context:

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
# #[cgp_type]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_type]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_component(MessageQuerier)]
# pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
#     fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
# }
#
# #[cgp_component(ApiBaseUrlGetter)]
# pub trait HasApiBaseUrl {
#     fn api_base_url(&self) -> &String;
# }
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
#[cgp_new_provider]
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

In addition to the base API URL, many API services require authentication to protect their resources from unauthorized access. For this example, we’ll use simple _bearer tokens_ for API access.

Just as we did with `HasApiBaseUrl`, we can define a `HasAuthToken` trait to retrieve the authentication token as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[cgp_type]
pub trait HasAuthTokenType {
    type AuthToken;
}

#[cgp_component(AuthTokenGetter)]
pub trait HasAuthToken: HasAuthTokenType {
    fn auth_token(&self) -> &Self::AuthToken;
}
```

Similar to the pattern used in the [earlier chapter](./associated-types.md), we first define `HasAuthTokenType` to keep the `AuthToken` type abstract. In fact, this `HasAuthTokenType` trait and its associated providers can be reused across different chapters or applications. This demonstrates how minimal CGP traits facilitate the reuse of interfaces in multiple contexts.

Next, we define a getter trait, `HasAuthToken`, which provides access to an abstract `AuthToken` value from the context. With this in place, we can now update `ReadMessageFromApi` to include the authentication token in the `Authorization` HTTP header:

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
# #[cgp_type]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_type]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component(MessageQuerier)]
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
#     provider: AuthTokenGetter,
# }]
# pub trait HasAuthToken: HasAuthTokenType {
#     fn auth_token(&self) -> &Self::AuthToken;
# }
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
#[cgp_new_provider]
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

In this updated code, we use the [`bearer_auth`](https://docs.rs/reqwest/latest/reqwest/blocking/struct.RequestBuilder.html#method.bearer_auth) method from the `reqwest` library to include the authentication token in the HTTP header. In this case, the provider only requires that `Context::AuthToken` implement the `Display` trait, allowing it to work with custom `AuthToken` types, not limited to `String`.

## Traits with Multiple Getter Methods

When creating providers like `ReadMessageFromApi`, which often need to use both `HasApiBaseUrl` and `HasAuthToken`, an alternative design would be to combine these two traits into a single one, containing both accessor methods:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
#[cgp_component(ApiClientFieldsGetter)]
pub trait HasApiClientFields: HasAuthTokenType {
    fn api_base_url(&self) -> &String;

    fn auth_token(&self) -> &Self::AuthToken;
}
```


While this approach works, it introduces unnecessary coupling between the `api_base_url` and `auth_token` fields. If a provider only requires `api_base_url` but not `auth_token`, it would still need to include the unnecessary `auth_token` dependency. Additionally, this design prevents us from implementing separate providers that could provide the `api_base_url` and `auth_token` fields independently, each with its own logic.

Furthermore, traits that contain only one method can benefit from the [`UseField`](./use-field-pattern.md) pattern that will be introduced later, which helps to simplify the boilerplate required to implement accessor traits, as well as allowing reusable accessor providers to be defined.

Ultimately, CGP does not prevent you from defining multiple accessor methods into one trait, or even mix the trait with other items such as associated types or non-getter methods. It is your own decision of how to design CGP traits. Just keep in mind that it is common in CGP to see accessor traits that each contain only one getter method.

## Implementing Accessor Providers

Now that we have implemented the provider, we would look at how to implement
a concrete context that uses `ReadMessageFromApi` and implement the accessors.
We can implement an `ApiClient` context that makes use of all providers
as follows:

```rust
# extern crate cgp;
# extern crate cgp_error_anyhow;
# extern crate reqwest;
# extern crate serde;
#
# use core::fmt::Display;
#
# use cgp::core::component::UseDelegate;
# use cgp::extra::error::RaiseFrom;
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeProviderComponent};
# use cgp::prelude::*;
# use cgp_error_anyhow::{DebugAnyhowError, UseAnyhowError};
# use reqwest::blocking::Client;
# use reqwest::StatusCode;
# use serde::Deserialize;
#
# #[cgp_type]
# pub trait HasMessageType {
#     type Message;
# }
#
# #[cgp_type]
# pub trait HasMessageIdType {
#     type MessageId;
# }
#
# #[cgp_type]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
# #[cgp_component(MessageQuerier)]
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
#     provider: AuthTokenGetter,
# }]
# pub trait HasAuthToken: HasAuthTokenType {
#     fn auth_token(&self) -> &Self::AuthToken;
# }
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
# #[cgp_new_provider]
# impl<Context> MessageQuerier<Context> for ReadMessageFromApi
# where
#     Context: HasMessageIdType<MessageId = u64>
#         + HasMessageType<Message = String>
#         + HasApiBaseUrl
#         + HasAuthToken
#         + CanRaiseError<reqwest::Error>
#         + CanRaiseError<ErrStatusCode>,
#     Context::AuthToken: Display,
# {
#     fn query_message(context: &Context, message_id: &u64) -> Result<String, Context::Error> {
#         let client = Client::new();
#
#         let url = format!("{}/api/messages/{}", context.api_base_url(), message_id);
#
#         let response = client
#             .get(url)
#             .bearer_auth(context.auth_token())
#             .send()
#             .map_err(Context::raise_error)?;
#
#         let status_code = response.status();
#
#         if !status_code.is_success() {
#             return Err(Context::raise_error(ErrStatusCode { status_code }));
#         }
#
#         let message_response: ApiMessageResponse = response.json().map_err(Context::raise_error)?;
#
#         Ok(message_response.message)
#     }
# }
#
#[cgp_context]
pub struct ApiClient {
    pub api_base_url: String,
    pub auth_token: String,
}

delegate_components! {
    ApiClientComponents {
        ErrorTypeProviderComponent:
            UseAnyhowError,
        ErrorRaiserComponent:
            UseDelegate<RaiseApiErrors>,
        MessageIdTypeProviderComponent:
            UseType<u64>,
        MessageTypeProviderComponent:
            UseType<String>,
        AuthTokenTypeProviderComponent:
            UseType<String>,
        MessageQuerierComponent:
            ReadMessageFromApi,
    }
}

delegate_components! {
    new RaiseApiErrors {
        reqwest::Error: RaiseFrom,
        ErrStatusCode: DebugAnyhowError,
    }
}

#[cgp_provider]
impl ApiBaseUrlGetter<ApiClient> for ApiClientComponents {
    fn api_base_url(api_client: &ApiClient) -> &String {
        &api_client.api_base_url
    }
}

#[cgp_provider]
impl AuthTokenGetter<ApiClient> for ApiClientComponents {
    fn auth_token(api_client: &ApiClient) -> &String {
        &api_client.auth_token
    }
}

pub trait CanUseApiClient: CanQueryMessage {}

impl CanUseApiClient for ApiClient {}
```

The `ApiClient` context is defined with the fields that we need to implement the accessor traits.
We then have context-specific implementation of `ApiBaseUrlGetter` and `AuthTokenGetter` to work
directly with `ApiClient`. With that, our wiring is completed, and we can check that
`ApiClient` implements `CanQueryMessage`.
