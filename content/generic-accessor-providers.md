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

The `GetAuthToken` provider is slightly more complex since the `auth_token` method returns an abstract `Context::AuthToken` type. To handle this, we require the `Context` to implement `HasAuthTokenType` and for the `Value` associated type to match `Context::AuthToken`. This ensures that `GetAuthToken` can be used with any context that has an `auth_token` field of the same type as the `AuthToken` defined in `HasAuthTokenType`.

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