use anyhow::Error;
use serde::{Deserialize, Serialize};

pub trait HasProvider {
    type Provider;
}

pub trait IsProviderFor<Component, Context, Params = ()> {}

pub trait DelegateComponent<Name> {
    type Delegate;
}

pub trait CanUseComponent<Component, Params = ()> {}

impl<Context, Component, Params> CanUseComponent<Component, Params> for Context
where
    Context: HasProvider,
    Context::Provider: IsProviderFor<Component, Context, Params>,
{
}

pub struct StringFormatterComponent;

pub struct StringParserComponent;

pub trait CanFormatToString {
    fn format_to_string(&self) -> Result<String, Error>;
}

pub trait CanParseFromString: Sized {
    fn parse_from_string(raw: &str) -> Result<Self, Error>;
}

pub trait StringFormatter<Context>:
    IsProviderFor<StringFormatterComponent, Context>
{
    fn format_to_string(context: &Context) -> Result<String, Error>;
}

pub trait StringParser<Context>:
    IsProviderFor<StringParserComponent, Context>
{
    fn parse_from_string(raw: &str) -> Result<Context, Error>;
}

impl<Context> CanFormatToString for Context
where
    Context: HasProvider,
    Context::Provider: StringFormatter<Context>,
{
    fn format_to_string(&self) -> Result<String, Error> {
        Context::Provider::format_to_string(self)
    }
}

impl<Context> CanParseFromString for Context
where
    Context: HasProvider,
    Context::Provider: StringParser<Context>,
{
    fn parse_from_string(raw: &str) -> Result<Context, Error> {
        Context::Provider::parse_from_string(raw)
    }
}

impl<Context, Component> StringFormatter<Context> for Component
where
    Component: DelegateComponent<StringFormatterComponent>
        + IsProviderFor<StringFormatterComponent, Context>,
    Component::Delegate: StringFormatter<Context>,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Component::Delegate::format_to_string(context)
    }
}

impl<Context, Component> StringParser<Context> for Component
where
    Component: DelegateComponent<StringParserComponent>
        + IsProviderFor<StringParserComponent, Context>,
    Component::Delegate: StringParser<Context>,
{
    fn parse_from_string(raw: &str) -> Result<Context, Error> {
        Component::Delegate::parse_from_string(raw)
    }
}

pub struct FormatAsJsonString;

impl<Context> StringFormatter<Context> for FormatAsJsonString
where
    Context: Serialize,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Ok(serde_json::to_string(context)?)
    }
}

impl<Context> IsProviderFor<StringFormatterComponent, Context>
    for FormatAsJsonString
where
    Context: Serialize,
{
}

pub struct ParseFromJsonString;

impl<Context> StringParser<Context> for ParseFromJsonString
where
    Context: for<'a> Deserialize<'a>,
{
    fn parse_from_string(json_str: &str) -> Result<Context, Error> {
        Ok(serde_json::from_str(json_str)?)
    }
}

impl<Context> IsProviderFor<StringParserComponent, Context>
    for ParseFromJsonString
where
    Context: for<'a> Deserialize<'a>,
{
}

// Note: We pretend to forgot to derive Serialize here
#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

pub struct PersonComponents;

impl HasProvider for Person {
    type Provider = PersonComponents;
}

impl DelegateComponent<StringFormatterComponent> for PersonComponents {
    type Delegate = FormatAsJsonString;
}

impl<Context> IsProviderFor<StringFormatterComponent, Context>
    for PersonComponents
where
    FormatAsJsonString: IsProviderFor<StringFormatterComponent, Context>,
{
}

impl DelegateComponent<StringParserComponent> for PersonComponents {
    type Delegate = ParseFromJsonString;
}

impl<Context> IsProviderFor<StringParserComponent, Context> for PersonComponents where
    ParseFromJsonString: IsProviderFor<StringParserComponent, Context>
{
}

pub trait CanUsePerson:
    CanUseComponent<StringFormatterComponent>
    + CanUseComponent<StringParserComponent>
{
}

impl CanUsePerson for Person {}
