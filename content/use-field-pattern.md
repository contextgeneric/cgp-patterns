# The `UseField` Pattern

In the previous chapter, we were able to implement context-generic accessor providers like `GetApiUrl` and `UseFields` without directly referencing the concrete context. However, the field names, such as `api_url` and `auth_token`, were hardcoded into the provider implementation. This means that a concrete context cannot choose different _field names_ for these specific fields unless it manually re-implements the accessors.

There are various reasons why a context might want to use different names for the field values. For instance, two independent accessor providers might choose the same field name for different types, or a context might have multiple similar fields with slightly different names. In these cases, it would be beneficial to allow the context to customize the field names instead of having the providers pick fixed field names.

To address this, the `cgp` crate provides the `UseField` marker type (note the lack of `s`, making it different from `UseFields`), which we can leverage to implement flexible accessor providers:

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
# #[cgp_component(ApiBaseUrlGetter)]
# pub trait HasApiBaseUrl {
#     fn api_base_url(&self) -> &String;
# }
#
# #[cgp_type]
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
#[cgp_provider]
impl<Context, Tag> ApiBaseUrlGetter<Context> for UseField<Tag>
where
    Context: HasField<Tag, Value = String>,
{
    fn api_base_url(context: &Context) -> &String {
        context.get_field(PhantomData)
    }
}

#[cgp_provider]
impl<Context, Tag> AuthTokenGetter<Context> for UseField<Tag>
where
    Context: HasAuthTokenType + HasField<Tag, Value = Context::AuthToken>,
{
    fn auth_token(context: &Context) -> &Context::AuthToken {
        context.get_field(PhantomData)
    }
}
```

Compared to `UseFields`, the implementation of `UseField` is parameterized by an additional `Tag` type, which represents the field name we want to access.
The structure of the implementation is almost the same as before, but instead of using `symbol!` to directly reference the field names, we rely on the `Tag` type to abstract the field names.

## Deriving `UseField` from `#[cgp_getter]`

The implementation of `UseField` on accessor traits can be automatically derived when the trait is defined with `#[cgp_getter]`. However, the derivation will only occur if the accessor trait contains exactly one accessor method. This is because, in cases with multiple methods, there is no clear way to determine which accessor method should utilize the `Tag` type specified in `UseField`.

By combining `#[cgp_getter]` with `UseField`, we can streamline the implementation of `ApiClient` and directly wire the accessor components within `delegate_components!`:

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
# use cgp::core::error::{ErrorRaiserComponent, ErrorTypeProviderComponent};
# use cgp::core::field::UseField;
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
#[cgp_getter]
pub trait HasApiBaseUrl {
    fn api_base_url(&self) -> &String;
}

#[cgp_getter]
pub trait HasAuthToken: HasAuthTokenType {
    fn auth_token(&self) -> &Self::AuthToken;
}

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
# #[cgp_context]
# #[derive(HasField)]
# pub struct ApiClient {
#     pub api_base_url: String,
#     pub auth_token: String,
# }
#
delegate_components! {
    ApiClientComponents {
        ErrorTypeProviderComponent: UseAnyhowError,
        ErrorRaiserComponent: UseDelegate<RaiseApiErrors>,
        MessageIdTypeProviderComponent: UseType<u64>,
        MessageTypeProviderComponent: UseType<String>,
        AuthTokenTypeProviderComponent: UseType<String>,
        ApiBaseUrlGetterComponent: UseField<symbol!("api_base_url")>,
        AuthTokenGetterComponent: UseField<symbol!("auth_token")>,
        MessageQuerierComponent: ReadMessageFromApi,
    }
}
#
# delegate_components! {
#     new RaiseApiErrors {
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
