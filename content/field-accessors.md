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

## Accessor Method Minimalism

Given that it is common for providers like `ReadMessageFromApi` to use both `HasApiBaseUrl` and
`HasAuthToken` together, it may be tempting to merge the two traits and define a single trait
that contains both accessor methods:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
#[cgp_component {
    provider: ApiClientFieldsGetter,
}]
pub trait HasApiClientFields: HasAuthTokenType {
    fn api_base_url(&self) -> &String;

    fn auth_token(&self) -> &Self::AuthToken;
}
```

Although this approach also works, it introduces unnecessary coupling between
the `api_base_url` field and the `auth_token` field.
If a provider only needs `api_base_url` but not `auth_token`, it would still
have to include the dependencies that it don't need.
Similarly, we can no longer implement separate providers for `ApiClientFieldsGetter`
to separately provide the fields `api_base_url` and `auth_token` in different ways.

The coupling of unrelated fields also makes it more challenging to evolve the
application in the future. For example, if we switch to a different authentication
method like public key cryptography, we now need to remove the `auth_token`
method and replace it with a different method, which would affect all code
that depend on `HasApiClientFields`. On the other hand, it is much simpler
to add an additional getter trait, and gradually deprecate and transition
providers to use the new trait while still keeping the old trait around.

As an application grows more complex, it would also be common to require
dozens of accessor methods, which would make a trait like `HasApiClientFields`
quickly become the bottleneck, and making it difficult for the application
to further evolve. In general, it is not possible to know up front which
of the accessor methods are related, and it can be a distraction to
attempt to make up theories of why it "makes sense" to group accessor
methods in certain ways.

With the experience of using CGP in real world applications, we find that
one accessor method per accessor trait is the most effective way to
quickly iterate on the application implementation.
This makes it easy to add or remove accessor methods, and it removes a lot of
cognitive overload on having to think, decide and debate about which trait
an accessor method should belong or not belong to.
With the passage of time, it is almost inevitable that an accessor trait
that contains multiple accessor methods will need to be broken up,
because some of the accessor methods are no longer applicable to some
part of the application.

As we will see in later sections and chapters, breaking the accessor methods
down to individual traits also allows us to introduce new design patterns
that can work when the trait contains only one accessor method.

Nevertheless, CGP does not prevent developers to define accessor traits that contain
multiple types and accessor methods.
In terms of comfort, it would also make sense for developers who are new to CGP
to want to define non-minimal traits, since it has been in the mainstream
programming practices for decades.
As a result, readers are encourage to feel free to experiment around, and
include as many types and methods in a CGP trait as they prefer.

On the other hand, for the purpose of this book, we will continue to make use
of minimal traits, since the book serves as reference materials that should
encourage best practices to its readers.

## Implementing Accessor Providers

Now that we have implemented the provider, we would look at how to implement
a concrete context that uses `ReadMessageFromApi` and implement the accessors.

First of all, we would implement the type traits by implementing type providers
that fit the constraints of `ReadMessageFromApi`:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
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
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
# }
#
pub struct UseU64MessageId;

impl<Context> ProvideMessageIdType<Context> for UseU64MessageId {
    type MessageId = u64;
}

pub struct UseStringMessage;

impl<Context> ProvideMessageType<Context> for UseStringMessage {
    type Message = String;
}

pub struct UseStringAuthToken;

impl<Context> ProvideAuthTokenType<Context> for UseStringAuthToken {
    type AuthToken = String;
}
```

We can then implement an `ApiClient` context that makes use of all providers
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
# use cgp::core::error::impls::RaiseFrom;
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
# use cgp::prelude::*;
# use cgp_error_anyhow::{DebugAnyhowError, UseAnyhowError};
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
# pub struct UseStringAuthToken;
#
# impl<Context> ProvideAuthTokenType<Context> for UseStringAuthToken {
#     type AuthToken = String;
# }
#
# pub struct UseU64MessageId;
#
# impl<Context> ProvideMessageIdType<Context> for UseU64MessageId {
#     type MessageId = u64;
# }
#
# pub struct UseStringMessage;
#
# impl<Context> ProvideMessageType<Context> for UseStringMessage {
#     type Message = String;
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
        MessageIdTypeComponent: UseU64MessageId,
        MessageTypeComponent: UseStringMessage,
        AuthTokenTypeComponent: UseStringAuthToken,
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

impl AuthTokenGettter<ApiClient> for ApiClientComponents {
    fn auth_token(api_client: &ApiClient) -> &String {
        &api_client.auth_token
    }
}

pub trait CanUseApiClient: CanQueryMessage {}

impl CanUseApiClient for ApiClient {}
```

The `ApiClient` context is defined with the fields that we need to implement the accessor traits.
We then have context-specific implementation of `ApiBaseUrlGetter` and `AuthTokenGettter` to work
directly with `ApiClient`. With that, our wiring is completed, and we can check that
`ApiClient` implements `CanQueryMessage`.

## Context-Generic Accessor Provider

Although the previous accessor implementation for `ApiClient` works, we have to have explicit and
concrete access to the `ApiClient` context in order to implement the accessors.
While this is not too bad with only two accessor methods, it can quickly become tedious once
the application grows, and we need to implement many accessors across many contexts.
It would be more efficient if we can implement _context-generic_ providers for field accessors,
and then use them for any context that contains a given field.

To make the implementation of context-generic accessors possible, the `cgp` crate offers a derivable
`HasField` trait that can be used as a proxy to access the fields in a concrete context.
The trait is defined as follows:

```rust
# use core::marker::PhantomData;
#
pub trait HasField<Tag> {
    type Value;

    fn get_field(&self, tag: PhantomData<Tag>) -> &Self::Value;
}
```

For each of the field inside a concrete context, we can implement a `HasField` instance
with the `Tag` type representing the field _name_, and the associated type `Value`
representing the field _type_.
There is also a `get_field` method, which gets a reference of the field value from
the context. The `get_field` method accepts an additional `tag` parameter,
which is just a `PhantomData` with the field name `Tag` as the type.
This phantom parameter is mainly used to help type inference in Rust,
as otherwise Rust would not be able to infer which field `Tag` we are trying to access.

We can automatically derive `HasField` instances for a context like `ApiClient`
by using the derive macro as follows:

```rust
# extern crate cgp;
#
# use cgp::prelude::*;
#
#[derive(HasField)]
pub struct ApiClient {
    pub api_base_url: String,
    pub auth_token: String,
}
```

The derive macro would then generate the following `HasField` instances for
`ApiClient`:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# pub struct ApiClient {
#     pub api_base_url: String,
#     pub auth_token: String,
# }
impl HasField<symbol!("api_base_url")> for ApiClient {
    type Value = String;

    fn get_field(&self, _tag: PhantomData<symbol!("api_base_url")>) -> &String {
        &self.api_base_url
    }
}

impl HasField<symbol!("auth_token")> for ApiClient {
    type Value = String;

    fn get_field(&self, _tag: PhantomData<symbol!("auth_token")>) -> &String {
        &self.auth_token
    }
}
```

## Symbols

In the derived `HasField` instances, we can see the use of `symbol!("api_base_url")`
and `symbol!("auth_token")` at the position of the `Tag` generic type.
Recall that a string like `"api_base_url"` is a _value_ of type `&str`,
but we want to use the string as a _type_.
To do that, we use the `symbol!` macro to "lift" a string value into a unique
type, so that we get a _type_ that uniquely identifies the string `"api_base_url"`.
Basically, this means that if the string content in two different uses of `symbol!`
are the same, then they would be treated as the same type.

Behind the scene, `symbol!` first use the `Char` type to "lift" individual characters
into types. The `Char` type is defined as follows:

```rust
pub struct Char<const CHAR: char>;
```

We make use of the [_const generics_](https://blog.rust-lang.org/2021/02/26/const-generics-mvp-beta.html)
feature in Rust to parameterize `Char` with a constant `CHAR` of type `char`.
The `Char` struct itself has an empty body, because we only want to use it like
a `char` at the type level.

Note that although we can use const generics to lift individual characters, we can't
yet use a type like `String` or `&str` inside const generics.
So until we can use strings inside const generics, we need a different workaround
to lift strings into types.

We workaround that by constructing a _type-level list_ of characters. So a type like
`symbol!("abc")` would be desugared to something like:

```rust,ignore
(Char<'a'>, (Char<'b'>, (Char<'c'>, ())))
```

In `cgp`, instead of using the native Rust tuple, we define the `Cons` and `Nil`
types to help identifying type level lists:

```rust
pub struct Nil;

pub struct Cons<Head, Tail>(pub Head, pub Tail);
```

Similar to the linked list concepts in Lisp, the `Nil` type is used to represent
an empty type-level list, and the `Cons` type is used to "add" an element to the
front of the type-level list.

With that, the actual desugaring of a type like `symbol!("abc")` looks like follows:

```rust,ignore
Cons<Char<'a'>, Cons<Char<'b'>, Cons<Char<'c'>, Nil>>>
```

Although the type make look complicated, it has a pretty compact representation from the
perspective of the Rust compiler. And since we never construct a value out of the symbol
type at runtime, we don't need to worry about any runtime overhead on using symbol types.
Aside from that, since we will mostly only use `HasField` to implement context-generic
accessors, there is negligible compile-time overhead of using `HasField` inside large
codebases.

It is also worth noting that the current representation of symbols is a temporary
workaround. Once Rust supports the use of strings inside const generics, we can
migrate the desugaring of `symbol!` to make use of that to simplify the type
representation.