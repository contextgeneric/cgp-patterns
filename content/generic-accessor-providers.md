# Context-Generic Accessor Providers

While the previous accessor implementation for `ApiClient` works, it requires explicit and concrete access to the `ApiClient` context to implement the accessors. While this approach is manageable with only a couple of accessor methods, it can quickly become cumbersome as the application grows and requires numerous accessors across multiple contexts. A more efficient approach would be to implement _context-generic_ providers for field accessors, allowing us to reuse them across any context that contains the relevant field.

To enable the implementation of context-generic accessors, the `cgp` crate provides a derivable `HasField` trait. This trait acts as a _proxy_, allowing access to fields in a concrete context. The trait is defined as follows:

```rust
# use core::marker::PhantomData;
#
pub trait HasField<Tag> {
    type Value;

    fn get_field(&self, tag: PhantomData<Tag>) -> &Self::Value;
}
```

For each field within a concrete context, we can implement a `HasField` instance by associating a `Tag` type with the field's _name_ and an associated type `Value` representing the field's _type_. Additionally, the `HasField` trait includes a `get_field` method, which retrieves a reference to the field value from the context. The `get_field` method accepts an additional `tag` parameter, which is a `PhantomData` type parameter tied to the field's name `Tag`. This phantom parameter helps with type inference in Rust, as without it, Rust would not be able to deduce which field associated with `Tag` is being accessed.

We can automatically derive `HasField` instances for a context like `ApiClient` using the derive macro, as shown below:

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

The derive macro would then generate the corresponding `HasField` instances for
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

In the derived `HasField` instances, we observe the use of `symbol!("api_base_url")` and `symbol!("auth_token")` for the `Tag` generic type. While a string like `"api_base_url"` is a value of type `&str`, we need to use it as a _type_ within the `Tag` parameter. To achieve this, we use the `symbol!` macro to "lift" a string value into a unique type, which allows us to treat the string `"api_base_url"` as a _type_. Essentially, this means that if the string content is the same across two uses of `symbol!`, the types will be treated as equivalent.

Behind the scenes, the `symbol!` macro first uses the `Char` type to "lift" individual characters into types. The `Char` type is defined as follows:

```rust
pub struct Char<const CHAR: char>;
```

This makes use of Rust's [_const generics_](https://blog.rust-lang.org/2021/02/26/const-generics-mvp-beta.html) feature to parameterize `Char` with a constant `CHAR` of type `char`. The `Char` struct itself is empty, as we only use it for type-level manipulation.

Although we can use const generics to lift individual characters, we currently cannot use a type like `String` or `&str` within const generics. As a workaround, we construct a _type-level list_ of characters. For example, `symbol!("abc")` is desugared to a type-level list of characters like:

```rust,ignore
(Char<'a'>, (Char<'b'>, (Char<'c'>, ())))
```

In `cgp`, instead of using Rust’s native tuple, we define the `Cons` and `Nil` types to represent type-level lists:

```rust
pub struct Nil;

pub struct Cons<Head, Tail>(pub Head, pub Tail);
```

The `Nil` type represents an empty type-level list, while `Cons` is used to prepend an element to the front of the list, similar to how linked lists work in Lisp.

Thus, the actual desugaring of `symbol!("abc")` looks like this:

```rust,ignore
Cons<Char<'a'>, Cons<Char<'b'>, Cons<Char<'c'>, Nil>>>
```

While this type may seem complex, it has a compact representation from the perspective of the Rust compiler. Furthermore, since we don’t construct values from symbol types at runtime, there is no runtime overhead associated with them. The use of `HasField` to implement context-generic accessors introduces negligible compile-time overhead, even in large codebases.

It’s important to note that the current representation of symbols is a temporary workaround. Once Rust supports using strings in const generics, we can simplify the desugaring process and adjust our implementation accordingly.

If the explanation here still feels unclear, think of symbols as strings being used as _types_ rather than values. In later sections, we’ll explore how `cgp` provides additional abstractions that abstract away the use of `symbol!` and `HasField`. These abstractions simplify the process, so you won’t need to worry about these details in simple cases.

## Auto Accessor Traits

The process of defining and wiring many CGP components can be overwhelming for developers who are new to CGP. In the early stages of a project, there is typically not much need for customizing how fields are accessed. As a result, some developers may find the full use of field accessors introduced in this chapter unnecessarily complex.

To simplify the use of accessor traits, one approach is to define them not as CGP components, but as regular Rust traits with blanket implementations that leverage `HasField`. For example, we can redefine the `HasApiBaseUrl` trait as follows:

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

With this approach, the `HasApiBaseUrl` trait will be automatically implemented for any context that derives `HasField` and contains the relevant field. There is no longer need for explicit wiring of the `ApiBaseUrlGetterComponent` within the context components.

This approach allows providers, such as `ReadMessageFromApi`, to still use accessor traits like `HasApiBaseUrl` to simplify field access. Meanwhile, context implementers can simply use `#[derive(HasField)]` without having to worry about manual wiring.

The main drawback of this approach is that the context cannot easily override the implementation of `HasApiBaseUrl`, unless it opts not to implement `HasField`. However, it would be straightforward to refactor the trait in the future to convert it into a full CGP component.

Overall, this approach may be an appealing option for developers who want a simpler experience with CGP without fully utilizing its advanced features.

## The `#[cgp_auto_getter]` Macro

To streamline the creation of auto accessor traits, the `cgp` crate provides the `#[cgp_auto_getter]` macro, which derives blanket implementations for accessor traits. For instance, the earlier example can be rewritten as follows:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# cgp_type!( AuthToken );
#
#[cgp_auto_getter]
pub trait HasApiBaseUrl {
    fn api_base_url(&self) -> &String;
}

#[cgp_auto_getter]
pub trait HasAuthToken: HasAuthTokenType {
    fn auth_token(&self) -> &Self::AuthToken;
}
```

Since `#[cgp_auto_getter]` generates a blanket implementation leveraging `HasField` directly, there is no corresponding provider trait being derived in this case.

The `#[cgp_auto_getter]` attribute can also be applied to accessor traits that define multiple getter methods. For instance, we can combine two accessor traits into one, as shown below:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# cgp_type!( AuthToken );
#
#[cgp_auto_getter]
pub trait HasApiClientFields: HasAuthTokenType {
    fn api_base_url(&self) -> &String;

    fn auth_token(&self) -> &Self::AuthToken;
}
```

By using `#[cgp_auto_getter]`, accessor traits are automatically implemented for contexts that use `#[derive(HasField)]` and include fields matching the names and return types of the accessor methods. This approach encapsulates the use of `HasField` and `symbol!`, providing well-typed and idiomatic interfaces for field access while abstracting the underlying mechanics.

## Using `HasField` in Accessor Providers

With `HasField`, we can implement context-generic providers like `ApiUrlGetter`. Here's an example:

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

In this implementation, `GetApiUrl` is defined for any `Context` type that implements `HasField<symbol!("api_url"), Value = String>`. This means that as long as the context uses `#[derive(HasField)]`, and has a field named `api_url` of type `String`, the `GetApiUrl` provider can be used with it.

Similarly, we can implement a context-generic provider for `AuthTokenGetter` as follows:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# cgp_type!( AuthToken );
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

The `GetAuthToken` provider is slightly more complex since the `auth_token` method returns an abstract `Context::AuthToken` type. To handle this, we require the `Context` to implement `HasAuthTokenType` and for the `Value` associated type to match `Context::AuthToken`. This ensures that `GetAuthToken` can be used with any context that has an `auth_token` field of the same type as the `AuthToken` defined in `HasAuthTokenType`.

## The `UseFields` Pattern

The providers `GetAuthToken` and `GetApiUrl` share a common characteristic: they implement accessor traits for any context type by utilizing `HasField`, with the field name corresponding to the accessor method name. To streamline this pattern, `cgp` provides the `UseFields` marker struct, which simplifies the implementation of such providers:

```rust
struct UseFields;
```

With `UseFields`, we can bypass the need to define custom provider structs and implement the logic directly on `UseFields`, as shown below:

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
# cgp_type!( AuthToken );
#
# #[cgp_component {
#     provider: AuthTokenGetter,
# }]
# pub trait HasAuthToken: HasAuthTokenType {
#     fn auth_token(&self) -> &Self::AuthToken;
# }
#
impl<Context> ApiBaseUrlGetter<Context> for UseFields
where
    Context: HasField<symbol!("api_url"), Value = String>,
{
    fn api_base_url(context: &Context) -> &String {
        context.get_field(PhantomData)
    }
}

impl<Context> AuthTokenGetter<Context> for UseFields
where
    Context: HasAuthTokenType + HasField<symbol!("auth_token"), Value = Context::AuthToken>,
{
    fn auth_token(context: &Context) -> &Context::AuthToken {
        context.get_field(PhantomData)
    }
}
```

## The `#[cgp_getter]` Macro

The `cgp` crate offers the `#[cgp_getter]` macro, which automatically derives implementations like `UseFields`. As an extension of `#[cgp_component]`, it provides the same interface and generates the same CGP component traits and blanket implementations.

With `#[cgp_getter]`, you can define accessor traits and seamlessly use `UseFields` directly in the component wiring, eliminating the need for manual implementations:

```rust
# extern crate cgp;
# extern crate cgp_error_anyhow;
# extern crate reqwest;
# extern crate serde;
#
# use core::fmt::Display;
#
# use cgp::core::component::UseDelegate;
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeComponent};
# use cgp::core::field::UseField;
# use cgp::extra::error::RaiseFrom;
# use cgp::prelude::*;
# use cgp_error_anyhow::{DebugAnyhowError, UseAnyhowError};
# use reqwest::blocking::Client;
# use reqwest::StatusCode;
# use serde::Deserialize;
#
# cgp_type!(Message);
# cgp_type!(MessageId);
# cgp_type!(AuthToken);
#
# #[cgp_component {
#     provider: MessageQuerier,
# }]
# pub trait CanQueryMessage: HasMessageIdType + HasMessageType + HasErrorType {
#     fn query_message(&self, message_id: &Self::MessageId) -> Result<Self::Message, Self::Error>;
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

impl HasProvider for ApiClient {
    type Provider = ApiClientComponents;
}

delegate_components! {
    ApiClientComponents {
        ErrorTypeComponent: UseAnyhowError,
        ErrorRaiserComponent: UseDelegate<RaiseApiErrors>,
        MessageTypeComponent: UseType<String>,
        MessageIdTypeComponent: UseType<u64>,
        AuthTokenTypeProviderComponent: UseType<String>,
        [
            ApiBaseUrlGetterComponent,
            AuthTokenGetterComponent,
        ]: UseFields,
        MessageQuerierComponent: ReadMessageFromApi,
    }
}
#
# pub struct RaiseApiErrors;
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

Compared to `#[cgp_auto_getter]`, `#[cgp_getter]` follows the same wiring process as other CGP components. To achieve the same outcome as `#[cgp_auto_getter]`, the only additional step required is delegating the getter component to UseFields within `delegate_components!`.

The primary advantage of using `#[cgp_getter]` is the ability to define custom accessor providers that can retrieve fields from the context in various ways, as we will explore in the next section.

Like `#[cgp_auto_getter]`, `#[cgp_getter]` can also be used with accessor traits containing multiple methods. This makes it easy to upgrade a trait, such as `HasApiClientFields`, to use `#[cgp_getter]` if custom accessor providers are needed in the future:

```rust
# extern crate cgp;
#
# use core::marker::PhantomData;
#
# use cgp::prelude::*;
#
# cgp_type!( AuthToken );
#
#[cgp_getter {
    provider: ApiClientFieldsGetter,
}]
pub trait HasApiClientFields: HasAuthTokenType {
    fn api_base_url(&self) -> &String;

    fn auth_token(&self) -> &Self::AuthToken;
}
```

## Static Accessors

One advantage of defining minimal accessor traits is that it allows the implementation of custom accessor providers that do not necessarily read field values from the context. For instance, we can create _static accessor_ providers that always return a global constant value.

Static accessors are useful when we want to hard-code values for a specific context. For example, we might define a production `ApiClient` context that always uses a fixed API URL:

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

In this example, the `UseProductionApiUrl` provider implements `ApiBaseUrlGetter` for any context type. Inside the `api_base_url` method, we define a `static` variable `BASE_URL` using `OnceLock<String>`. This allows us to initialize the global variable exactly once, and it remains constant throughout the application.

[`OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) is especially useful since constructors like `String::from` are not `const` fn in Rust. By using `OnceLock::get_or_init`, we can run non-const constructors at runtime while still benefiting from compile-time guarantees. The static variable is scoped within the method, so it is only accessible and initialized by the provider.

With `UseProductionApiUrl`, we can now define a production `ApiClient` context, as shown below:

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
# pub struct GetAuthToken;
#
# impl<Context> AuthTokenGetter<Context> for GetAuthToken
# where
#     Context: HasAuthTokenType + HasField<symbol!("auth_token"), Value = Context::AuthToken>,
# {
#     fn auth_token(context: &Context) -> &Context::AuthToken {
#         context.get_field(PhantomData)
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
impl HasProvider for ApiClient {
    type Provider = ApiClientComponents;
}

delegate_components! {
    ApiClientComponents {
        ErrorTypeComponent: UseAnyhowError,
        ErrorRaiserComponent: UseDelegate<RaiseApiErrors>,
        MessageIdTypeComponent: UseType<u64>,
        MessageTypeComponent: UseType<String>,
        AuthTokenTypeProviderComponent: UseType<String>,
        ApiBaseUrlGetterComponent: UseProductionApiUrl,
        AuthTokenGetterComponent: GetAuthToken,
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

In the component wiring, we specify `UseProductionApiUrl` as the provider for `ApiBaseUrlGetterComponent`. Notably, the `ApiClient` context no longer contains the `api_base_url` field.

Static accessors are particularly useful for implementing specialized contexts where certain fields must remain constant. With this approach, constant values don't need to be passed around as part of the context during runtime, and there's no concern about incorrect values being assigned at runtime. Additionally, because of the compile-time wiring, this method may offer performance benefits compared to passing dynamic values during execution.

## Using `HasField` Directly Inside Providers

Since the `HasField` trait can be automatically derived by contexts, some developers may be tempted to forgo defining accessor traits and instead use `HasField` directly within the providers. For example, one could remove `HasApiBaseUrl` and `HasAuthToken` and implement `ReadMessageFromApi` as follows:

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

In the example above, the provider `ReadMessageFromApi` requires the context to implement `HasField<symbol!("api_base_url")>` and `HasField<symbol!("auth_token")>`. To preserve the original behavior, we add constraints ensuring that the `api_base_url` field is of type `String` and that the `auth_token` field matches the type of `Context::AuthToken`.

When using `get_field`, since there are multiple `HasField` instances in scope, we need to fully qualify the field access to specify which field we want to retrieve. For example, we call `context.get_field(PhantomData::<symbol!("api_base_url")>)` to access the `api_base_url` field.

However, while the direct use of `HasField` is possible, it does not necessarily simplify the code. In fact, it often requires more verbose specifications for each field. Additionally, using `HasField` directly necessitates explicitly defining the field types. In contrast, with custom accessor traits like `HasAuthToken`, we can specify that a method returns an abstract type like `Self::AuthToken, which prevents accidental access to fields with the same underlying concrete type.

Using `HasField` directly also makes the provider less flexible if the context requires custom access methods. For instance, if we wanted to put the `api_base_url` field inside a separate `ApiConfig` struct, we would run into difficulties with `HasField`:

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

In this case, an accessor trait like `HasApiUrl` would allow the context to easily use a custom accessor provider. With direct use of `HasField`, however, indirect access would be more cumbersome to implement.

That said, using `HasField` directly can be convenient during the initial development stages, as it reduces the number of traits a developer needs to manage. Therefore, we encourage readers to use `HasField` where appropriate and gradually migrate to more specific accessor traits when necessary.
