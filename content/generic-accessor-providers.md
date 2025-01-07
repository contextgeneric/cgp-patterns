# Context-Generic Accessor Providers

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

## Using `HasField` in Accessor Providers

Using `HasField`, we can then implement a context-generic provider for `ApiUrlGetter`
like follows:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# #[cgp_component {
#     provider: ApiBaseUrlGetter,
# }]
# pub trait HasApiBaseUrl {
#     fn api_base_url(&self) -> &String;
# }
#
pub struct GetApiUrl;

impl<Context> ApiBaseUrlGetter<Context> for GetApiUrl
where
    Context: HasField<symbol!("api_url"), Value = String>,
{
    fn api_base_url(context: &Context) -> &String {
        context.get_field(PhantomData)
    }
}
```

The provider `GetApiUrl` is implemented for any `Context` type that implements
`HasField<symbol!("api_url"), Value = String>`. This means that as long as the
context uses `#[derive(HasField)]` has an `api_url` field with `String` type,
then we can use `GetApiUrl` with it.

Similarly, we can implement a context-generic provider for `AuthTokenGetter` as follows:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
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
# #[cgp_component {
#     provider: AuthTokenGetter,
# }]
# pub trait HasAuthToken: HasAuthTokenType {
#     fn auth_token(&self) -> &Self::AuthToken;
# }
#
pub struct GetAuthToken;

impl<Context> AuthTokenGetter<Context> for GetAuthToken
where
    Context: HasAuthTokenType + HasField<symbol!("auth_token"), Value = Context::AuthToken>,
{
    fn auth_token(context: &Context) -> &Context::AuthToken {
        context.get_field(PhantomData)
    }
}
```

The provider `GetAuthToken` is slightly more complicated, because the `auth_token()` method
returns an abstract `Context::AuthToken` type.
To work with that, we first need `Context` to implement `HasAuthTokenType`, and then
require the `Value` associated type to be the same as `Context::AuthToken`.
This means that `GetAuthToken` can be used with a context, if it uses
`#[derive(HasField)]` and has an `auth_token` field with the same type as
the `AuthToken` type that it implements.

## The `UseField` Pattern

In the previous section, we managed to implement the context-generic accessor providers
`GetApiUrl` and `GetAuthToken`, without access to the concrete context. However, the field names
`api_url` and `auth_token` are hardcoded into the provider implementation. This means that
a concrete context cannot choose different _field names_ for the specific fields, unless
they manually re-implement the accessors.

There may be different reasons why a context may want to use different names to store the
field values. For example, there could be two independent accessor providers that happen
to choose the same field name for different types. A context may also have multiple similar
fields that serve similar purposes but with slightly different names.
Whatever the reason is, it would be nice if we can allow the contexts to customize the
field names, instead of letting the providers to pick fixed field names.

For this purpose, the `cgp` crate provides the `UseField` type that we can use to
implement accessor providers:

```rust
# use core::marker::PhantomData;
#
pub struct UseField<Tag>(pub PhantomData<Tag>);
```

Similar to the [`UseDelegate` pattern](./delegated-error-raiser.md), the `UseField` type
is used as a label for accessor implementations following the `UseField` pattern.
Using `UseField`, we can implement the providers as follows:


```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
# use cgp::core::field::UseField;
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
#     provider: AuthTokenGetter,
# }]
# pub trait HasAuthToken: HasAuthTokenType {
#     fn auth_token(&self) -> &Self::AuthToken;
# }
#
impl<Context, Tag> ApiBaseUrlGetter<Context> for UseField<Tag>
where
    Context: HasField<Tag, Value = String>,
{
    fn api_base_url(context: &Context) -> &String {
        context.get_field(PhantomData)
    }
}

impl<Context, Tag> AuthTokenGetter<Context> for UseField<Tag>
where
    Context: HasAuthTokenType + HasField<Tag, Value = Context::AuthToken>,
{
    fn auth_token(context: &Context) -> &Context::AuthToken {
        context.get_field(PhantomData)
    }
}
```

Compared to the explicit providers `GetApiUrl` and `GetAuthToken`, we implement
the traits `ApiBaseUrlGetter` and `AuthTokenGetter` directly on the `UseField`
type provided by the `cgp` crate.
The implementation is also parameterized by an additional `Tag` type, to represent
the name of the field we want to use.
We can see that the implementation is almost the same as before, except that
we no longer use `symbol!` to directly refer to the field names.

Using `UseField`, we get to simplify the implementation of `ApiClient` and
wire up the accessor components directly inside `delegate_components!`:

```rust
# extern crate cgp;
# extern crate cgp_error_anyhow;
# extern crate reqwest;
# extern crate serde;
#
# use core::fmt::Display;
# use core::marker::PhantomData;
#
# use cgp::core::component::UseDelegate;
# use cgp::extra::error::RaiseFrom;
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
# use cgp::core::field::UseField;
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
# impl<Context, Tag> ApiBaseUrlGetter<Context> for UseField<Tag>
# where
#     Context: HasField<Tag, Value = String>,
# {
#     fn api_base_url(context: &Context) -> &String {
#         context.get_field(PhantomData)
#     }
# }
#
# impl<Context, Tag> AuthTokenGetter<Context> for UseField<Tag>
# where
#     Context: HasAuthTokenType + HasField<Tag, Value = Context::AuthToken>,
# {
#     fn auth_token(context: &Context) -> &Context::AuthToken {
#         context.get_field(PhantomData)
#     }
# }
#
# #[derive(HasField)]
# pub struct ApiClient {
#     pub api_base_url: String,
#     pub auth_token: String,
# }
#
# pub struct ApiClientComponents;
#
# pub struct RaiseApiErrors;
#
# impl HasComponents for ApiClient {
#     type Components = ApiClientComponents;
# }
#
delegate_components! {
    ApiClientComponents {
        ErrorTypeComponent: UseAnyhowError,
        ErrorRaiserComponent: UseDelegate<RaiseApiErrors>,
        MessageIdTypeComponent: UseU64MessageId,
        MessageTypeComponent: UseStringMessage,
        AuthTokenTypeComponent: UseStringAuthToken,
        ApiBaseUrlGetterComponent: UseField<symbol!("api_base_url")>,
        AuthTokenGetterComponent: UseField<symbol!("auth_token")>,
        MessageQuerierComponent: ReadMessageFromApi,
    }
}
#
# delegate_components! {
#     RaiseApiErrors {
#         reqwest::Error: RaiseFrom,
#         ErrStatusCode: DebugAnyhowError,
#     }
# }
#
# pub trait CanUseApiClient: CanQueryMessage {}
#
# impl CanUseApiClient for ApiClient {}
```

The wiring above uses `UseField<symbol!("api_base_url")>` to implement `ApiBaseUrlGetterComponent`,
and `UseField<symbol!("auth_token")>` to implement `AuthTokenGetterComponent`.
With the field names specified explicitly in the wiring, we can easily change the field
names in the `ApiClient` context, and update the wiring accordingly.

## Using `HasField` Directly Inside Providers

Since the `HasField` trait can be automatically derived by contexts, some readers may be
tempted to not define any accessor trait, and instead make use of `HasField` directly
inside the providers. For example, we can in principle remove `HasApiBaseUrl` and
`HasAuthToken`, and re-implement `ReadMessageFromApi` as follows:

```rust
# extern crate cgp;
# extern crate reqwest;
# extern crate serde;
#
# use core::fmt::Display;
# use core::marker::PhantomData;
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
#     name: AuthTokenTypeComponent,
#     provider: ProvideAuthTokenType,
# }]
# pub trait HasAuthTokenType {
#     type AuthToken;
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
        + HasAuthTokenType
        + HasField<symbol!("api_base_url"), Value = String>
        + HasField<symbol!("auth_token"), Value = Context::AuthToken>
        + CanRaiseError<reqwest::Error>
        + CanRaiseError<ErrStatusCode>,
    Context::AuthToken: Display,
{
    fn query_message(context: &Context, message_id: &u64) -> Result<String, Context::Error> {
        let client = Client::new();

        let url = format!(
            "{}/api/messages/{}",
            context.get_field(PhantomData::<symbol!("api_base_url")>),
            message_id
        );

        let response = client
            .get(url)
            .bearer_auth(context.get_field(PhantomData::<symbol!("auth_token")>))
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

In the implementation above, the provider `ReadMessageFromApi` requires the context to implement
`HasField<symbol!("api_base_url")>` and `HasField<symbol!("auth_token")>`.
To preserve the original behavior, we also have additional constraints that the field `api_base_url`
needs to be of `String` type, and the field `auth_token` needs to have the same type as
`Context::AuthToken`.
When using `get_field`, since there are two instances of `HasField` implemented in scope,
we need to fully qualify the call to specify the field name that we want to access,
such as `context.get_field(PhantomData::<symbol!("api_base_url")>)`.

As we can see, the direct use of `HasField` may not necessary make the code simpler, and instead
require more verbose specification of the fields. The direct use of `HasFields` also requires
explicit specification of what the field types should be.
Whereas in accessor traits like `HasAuthToken`, we can better specify that the method always
return the abstract type `Self::AuthToken`, so one cannot accidentally read from different
fields that happen to have the same underlying concrete type.

By using `HasField` directly, the provider also makes it less flexible for the context to have
custom ways of getting the field value. For example, instead of putting the `api_url` field
directly in the context, we may want to put it inside another `ApiConfig` struct such as follows:

```rust
pub struct Config {
    pub api_base_url: String,
    // other fields
}

pub struct ApiClient {
    pub config: Config,
    pub auth_token: String,
    // other fields
}
```

In such cases, with an accessor trait like `HasApiUrl`, the context can easily make use of
custom accessor providers to implement such indirect access. But with direct use of
`HasFields`, it would be more tedious to implement the indirect access.

That said, similar to other shortcut methods, the direct use of `HasField` can be convenient
during initial development, as it helps to significantly reduce the number of traits the
developer needs to keep track of. As a result, we encourage readers to feel free to make
use of `HasField` as they see fit, and then slowly migrate to proper accessor traits
when the need arise.

## Static Accessors

One benefit of defining minimal accessor traits is that we get to implement custom
accessor providers that do not necessarily need to read the field values from the context.
For example, we can implement _static accessor_ providers that always return a global
constant value.

The use of static accessors can be useful when we want to hard code some values for a
specific context. For instance, we may want to define a production `ApiClient` context
that always use a hard-coded API URL:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
use std::sync::OnceLock;

# use cgp::prelude::*;
#
# #[cgp_component {
#     provider: ApiBaseUrlGetter,
# }]
# pub trait HasApiBaseUrl {
#     fn api_base_url(&self) -> &String;
# }
#
pub struct UseProductionApiUrl;

impl<Context> ApiBaseUrlGetter<Context> for UseProductionApiUrl {
    fn api_base_url(_context: &Context) -> &String {
        static BASE_URL: OnceLock<String> = OnceLock::new();

        BASE_URL.get_or_init(|| "https://api.example.com".into())
    }
}
```

The provider `UseProductionApiUrl` implements `ApiBaseUrlGetter` for any context type.
Inside the `api_base_url` method, we first define a static `BASE_URL` value with the
type `OnceLock<String>`. The use of [`OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
allows us to define a global variable in Rust that is initialized exactly once, and
then remain constant throughout the application.
This is mainly useful because constructors like `String::from` are not currently `const fn`,
so we have to make use of `OnceLock::get_or_init` to run the non-const constructor.
By defining the static variable inside the method, we ensure that the variable can only be
accessed and initialized by the provider.

Using `UseProductionApiUrl`, we can now define a production `ApiClient` context such as follows:

```rust
# extern crate cgp;
# extern crate cgp_error_anyhow;
# extern crate reqwest;
# extern crate serde;
#
# use core::fmt::Display;
# use core::marker::PhantomData;
# use std::sync::OnceLock;
#
# use cgp::core::component::UseDelegate;
# use cgp::extra::error::RaiseFrom;
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
# use cgp::core::field::UseField;
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
# impl<Context, Tag> AuthTokenGetter<Context> for UseField<Tag>
# where
#     Context: HasAuthTokenType + HasField<Tag, Value = Context::AuthToken>,
# {
#     fn auth_token(context: &Context) -> &Context::AuthToken {
#         context.get_field(PhantomData)
#     }
# }
#
# pub struct UseProductionApiUrl;
#
# impl<Context> ApiBaseUrlGetter<Context> for UseProductionApiUrl {
#     fn api_base_url(_context: &Context) -> &String {
#         static BASE_URL: OnceLock<String> = OnceLock::new();
#
#         BASE_URL.get_or_init(|| "https://api.example.com".into())
#     }
# }
#
#[derive(HasField)]
pub struct ApiClient {
    pub auth_token: String,
}

pub struct ApiClientComponents;

# pub struct RaiseApiErrors;
#
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
        ApiBaseUrlGetterComponent: UseProductionApiUrl,
        AuthTokenGetterComponent: UseField<symbol!("auth_token")>,
        MessageQuerierComponent: ReadMessageFromApi,
    }
}
#
# delegate_components! {
#     RaiseApiErrors {
#         reqwest::Error: RaiseFrom,
#         ErrStatusCode: DebugAnyhowError,
#     }
# }
#
# pub trait CanUseApiClient: CanQueryMessage {}
#
# impl CanUseApiClient for ApiClient {}
```

Inside the component wiring, we choose `UseProductionApiUrl` to be the provider
for `ApiBaseUrlGetterComponent`.
Notice that now the `ApiClient` context no longer contain any `api_base_url` field.

The use of static accessors can be useful to implement specialized contexts
that keep the values constant for certain fields.
With this approach, the constant values no longer needs to be passed around
as part of the context during runtime, and we no longer need to worry
about keeping the field private or preventing the wrong value being assigned
at runtime.
Thanks to the compile-time wiring, we may even get some performance advantage
as compared to passing around dynamic values at runtime.

## Auto Accessor Traits

The need to define and wire up many CGP components may overwhelm a developer who
is new to CGP.
At least during the beginning phase, a project don't usually that much flexibility
in customizing how fields are accessed.
As such, some may consider the full use of field accessors introduced in this chapter
being unnecessarily complicated.

One intermediate way to simplify use of accessor traits is to define them _not_
as CGP components, but as regular Rust traits with blanket implementations that
use `HasField`. For example, we can re-define the `HasApiUrl` trait as follows:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
pub trait HasApiBaseUrl {
    fn api_base_url(&self) -> &String;
}

impl<Context> HasApiBaseUrl for Context
where
    Context: HasField<symbol!("api_base_url"), Value = String>,
{
    fn api_base_url(&self) -> &String {
        self.get_field(PhantomData)
    }
}
```

This way, the `HasApiBaseUrl` will always be implemented for any context
that derive `HasField` and have the relevant field, and
there is no need to have explicit wiring of `ApiBaseUrlGetterComponent`
inside the wiring of the context components.

With this, providers like `ReadMessageFromApi` can still use traits like `HasApiBaseUrl`
to simplify the access of fields. And the context implementors can just use
`#[derive(HasField)]` without having to worry about the wiring.

The main downside of this approach is that the context cannot easily override the
implementation of `HaswApiBaseUrl`, unless they don't implement `HasField` at all.
Nevertheless, it will be straightforward to refactor the trait in the future
to turn it into a full CGP component.

As a result, this may be an appealing option for readers who want to have a simpler
experience of using CGP and not use its full power.

## Conclusion

In this chapter, we have learned about different ways to define accessor traits,
and to implement the accessor providers. The use of a derivable `HasField` trait
makes it possible to implement context-generic accessor providers without
requiring direct access to the concrete context. The use of the `UseField` pattern
unifies the convention of implementing field accessors, and allows contexts
to choose different field names for the accessors.

As we will see in later chapters, the use of context-generic accessor providers
make it possible to implement almost everything as context-generic providers,
and leaving almost no code tied to specific concrete contexts.