# Field Accessors

With impl-side dependencies, CGP offers a way to inject dependencies into providers without cluttering the public interfaces with extra constraints. One common use of this dependency injection is for a provider to retrieve values from the context. This pattern is often referred to as a field _accessor_ or _getter_, since it involves accessing field values from the context. In this chapter, we'll explore how to define and use field accessors effectively with CGP.

## Example: API Call

Suppose our application needs to make API calls to an external service to read messages by their message ID. To abstract away the details of the API call, we can define CGP traits as follows:

```rust
# extern crate cgp;
#
use cgp::prelude::*;

cgp_type!( Message );
cgp_type!( MessageId );

#[cgp_component {
    provider: MessageQuerier,
}]
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
# cgp_type!( Message );
# cgp_type!( MessageId );
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
#[cgp_component {
    provider: ApiBaseUrlGetter,
}]
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
# cgp_type!( Message );
# cgp_type!( MessageId );
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

In addition to the base API URL, many API services require authentication to protect their resources from unauthorized access. For this example, we’ll use simple _bearer tokens_ for API access.

Just as we did with `HasApiBaseUrl`, we can define a `HasAuthToken` trait to retrieve the authentication token as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
cgp_type!( AuthToken );

#[cgp_component {
    provider: AuthTokenGetter,
}]
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
# cgp_type!( Message );
# cgp_type!( MessageId );
# cgp_type!( AuthToken );
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
#     provider: AuthTokenGetter,
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

In this updated code, we use the [`bearer_auth`](https://docs.rs/reqwest/latest/reqwest/blocking/struct.RequestBuilder.html#method.bearer_auth) method from the `reqwest` library to include the authentication token in the HTTP header. In this case, the provider only requires that `Context::AuthToken` implement the `Display` trait, allowing it to work with custom `AuthToken` types, not limited to `String`.

## Accessor Method Minimalism

When creating providers like `ReadMessageFromApi`, which often need to use both `HasApiBaseUrl` and `HasAuthToken`, it might seem tempting to combine these two traits into a single one, containing both accessor methods:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# cgp_type!( AuthToken );
#
#[cgp_component {
    provider: ApiClientFieldsGetter,
}]
pub trait HasApiClientFields: HasAuthTokenType {
    fn api_base_url(&self) -> &String;

    fn auth_token(&self) -> &Self::AuthToken;
}
```

While this approach works, it introduces unnecessary coupling between the `api_base_url` and `auth_token` fields. If a provider only requires `api_base_url` but not `auth_token`, it would still need to include the unnecessary `auth_token` dependency. Additionally, this design prevents us from implementing separate providers that could provide the `api_base_url` and `auth_token` fields independently, each with its own logic.

This coupling also makes future changes more challenging. For example, if we switch to a different authentication method, like public key cryptography, we would need to remove the auth_token method and replace it with a new one. This change would affect all code dependent on `HasApiClientFields`. Instead, it's much easier to add a new getter trait and gradually transition providers to the new trait while keeping the old one intact.

As applications grow in complexity, it’s common to need many accessor methods. A trait like `HasApiClientFields`, with dozens of methods, could quickly become a bottleneck, making the application harder to evolve. Moreover, it's often unclear upfront which accessor methods are related, and trying to theorize about logical groupings can be a distraction.

From real-world experience using CGP, we’ve found that defining one accessor method per trait is the most effective approach for rapidly iterating on application development. This method simplifies the process of adding or removing accessor methods and reduces cognitive overload, as developers don’t need to spend time deciding or debating which method should belong to which trait. Over time, it's almost inevitable that a multi-method accessor trait will need to be broken up as some methods become irrelevant to parts of the application.

In future chapters, we’ll explore how breaking accessor methods down into individual traits can enable new design patterns that work well with single-method traits.

However, CGP doesn’t prevent developers from creating accessor traits with multiple methods and types. For those new to CGP, it might feel more comfortable to define non-minimal traits, as this has been a mainstream practice in programming for decades. So, feel free to experiment and include as many types and methods in a CGP trait as you prefer.

As an alternative to defining multiple accessor methods, you could define an inner struct containing all the common fields you’ll use across most providers:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
pub struct ApiClientFields {
    pub api_base_url: String,
    pub auth_token: String,
}

#[cgp_component {
    provider: ApiClientFieldsGetter,
}]
pub trait HasApiClientFields {
    fn api_client_fields(&self) -> &ApiClientFields;
}
```

In this example, we define an `ApiClientFields` struct that groups both the `api_base_url` and `auth_token` fields. The `HasApiClientFields` trait now only needs one getter method, returning the `ApiClientFields` struct.

One downside to this approach is that we can no longer use abstract types within the struct. For instance, the `ApiClientFields` struct stores the `auth_token` as a concrete `String` rather than as an abstract `AuthToken` type. As a result, this approach works best when your providers don’t rely on abstract types for their fields.

For the purposes of this book, we will continue to use minimal traits, as this encourages best practices and provides readers with a clear reference for idiomatic CGP usage.

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
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
# use cgp::prelude::*;
# use cgp_error_anyhow::{DebugAnyhowError, UseAnyhowError};
# use reqwest::blocking::Client;
# use reqwest::StatusCode;
# use serde::Deserialize;
#
# cgp_type!( Message );
# cgp_type!( MessageId );
# cgp_type!( AuthToken );
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
#     provider: AuthTokenGetter,
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
        MessageQuerierComponent: ReadMessageFromApi,
    }
}

delegate_components! {
    RaiseApiErrors {
        reqwest::Error: RaiseFrom,
        ErrStatusCode: DebugAnyhowError,
    }
}

impl ApiBaseUrlGetter<ApiClient> for ApiClientComponents {
    fn api_base_url(api_client: &ApiClient) -> &String {
        &api_client.api_base_url
    }
}

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
