# The `UseField` Pattern

In the previous section, we were able to implement context-generic accessor providers like `GetApiUrl` and `GetAuthToken` without directly referencing the concrete context. However, the field names, such as `api_url` and `auth_token`, were hardcoded into the provider implementation. This means that a concrete context cannot choose different _field names_ for these specific fields unless it manually re-implements the accessors.

There are various reasons why a context might want to use different names for the field values. For instance, two independent accessor providers might choose the same field name for different types, or a context might have multiple similar fields with slightly different names. In these cases, it would be beneficial to allow the context to customize the field names instead of having the providers pick fixed field names.

To address this, the `cgp` crate provides the `UseField` type, which we can leverage to implement flexible accessor providers:

```rust
# use core::marker::PhantomData;
#
pub struct UseField<Tag>(pub PhantomData<Tag>);
```

Similar to the [`UseDelegate` pattern](./delegated-error-raiser.md), the `UseField` type acts as a marker for accessor implementations that follow the UseField pattern. Using `UseField`, we can define the providers as follows:

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

In contrast to the explicit providers `GetApiUrl` and `GetAuthToken`, we now implement the `ApiBaseUrlGetter` and `AuthTokenGetter` traits directly on the `UseField` type provided by the `cgp` crate. The implementation is parameterized by an additional `Tag` type, which represents the field name we want to access.

The structure of the implementation is almost the same as before, but instead of using `symbol!` to directly reference the field names, we rely on the `Tag` type to abstract the field names.

By using `UseField`, we can simplify the implementation of `ApiClient` and wire up the accessor components directly within `delegate_components!`:

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

In this wiring example, `UseField<symbol!("api_base_url")>` is used to implement the `ApiBaseUrlGetterComponent`, and `UseField<symbol!("auth_token")>` is used for the `AuthTokenGetterComponent`. By explicitly specifying the field names in the wiring, we can easily change the field names in the `ApiClient` context and update the wiring accordingly.

## Conclusion

In this chapter, we explored various ways to define accessor traits and implement accessor providers. The `HasField` trait, being derivable, offers a way to create context-generic accessor providers without directly accessing the context's concrete fields. The `UseField` pattern standardizes how field accessors are implemented, enabling contexts to customize field names for the accessors.

As we will see in later chapters, context-generic accessor providers allow us to implement a wide range of functionality without tying code to specific concrete contexts. This approach makes it possible to maintain flexibility and reusability across different contexts.
