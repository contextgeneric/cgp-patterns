# Provider Delegation

In the previous chapter, we learned to make use of the `HasComponent` trait
to define a blanket implementation for a consumer trait like `CanFormatString`,
so that a context would automatically delegate the implementation to a provider
trait like `StringFormatter`. However, because there can only be one `Component`
type defined for `HasComponent`, this means that the given provider needs to
implement _all_ provider traits that we would like to use for the context.

In this chapter, we will learn to combine multiple providers that each implements
a distinct provider trait, and turn them into a single provider that implements
multiple provider traits.

## Implementing Multiple Providers

Consider that instead of just formatting a context as string, we also want to
parse the context from string. In CGP, we would define two separate traits to
handle the functionalities separately:

```rust
# extern crate anyhow;
#
use anyhow::Error;

pub trait CanFormatToString {
    fn format_to_string(&self) -> Result<String, Error>;
}

pub trait CanParseFromString: Sized {
    fn parse_from_string(raw: &str) -> Result<Self, Error>;
}
```

Similar to the previous chapter, we define `CanFormatToString` for formatting
a context into string, and `CanParseFromString` for parsing a context from a
string. Notice that `CanParseFromString` also has an additional `Sized`
constraint, as by default the `Self` type in Rust traits do not implement `Sized`,
to allow traits to be used in `dyn` trait objects.
Compared to before, we also make the methods return a `Result` to
handle errors during formatting and parsing. [^error] [^encoding]

Next, we also define the provider traits as follows:

```rust
# extern crate anyhow;
#
# use anyhow::Error;
#
# pub trait HasComponents {
#     type Components;
# }
#
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
# }
#
pub trait StringFormatter<Context> {
    fn format_to_string(context: &Context) -> Result<String, Error>;
}

pub trait StringParser<Context> {
    fn parse_from_string(raw: &str) -> Result<Context, Error>;
}

impl<Context> CanFormatToString for Context
where
    Context: HasComponents,
    Context::Components: StringFormatter<Context>,
{
    fn format_to_string(&self) -> Result<String, Error> {
        Context::Components::format_to_string(self)
    }
}

impl<Context> CanParseFromString for Context
where
    Context: HasComponents,
    Context::Components: StringParser<Context>,
{
    fn parse_from_string(raw: &str) -> Result<Context, Error> {
        Context::Components::parse_from_string(raw)
    }
}
```

Similar to the previous chapter, we make use of blanket implementations
and `HasComponents` to link the consumer traits `CanFormatToString`
and `CanParseFromString` with their respective provider traits, `StringFormatter`
and `StringParser`.

We can then implement context-generic providers for the given provider traits,
such as to format and parse the context as JSON if the context implements
`Serialize` and `Deserialize`:

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
# pub trait StringParser<Context> {
#     fn parse_from_string(raw: &str) -> Result<Context, Error>;
# }
#
use serde::{Serialize, Deserialize};

pub struct FormatAsJsonString;

impl<Context> StringFormatter<Context> for FormatAsJsonString
where
    Context: Serialize,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Ok(serde_json::to_string(context)?)
    }
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
```

The provider `FormatAsJsonString` implements `StringFormatter` for any
`Context` type that implements `Serialize`, and uses `serde_json::to_string`
to format the context as JSON. Similarly, the provider `ParseFromJsonString`
implements `StringParser` for any `Context` that implements `Deserialize`,
and parse the context from a JSON string.

[^error]: A proper introduction to error handling using CGP will be covered in
[future chapters](./error-handling.md). But for now, we will use use
[`anyhow::Error`](https://docs.rs/anyhow/latest/anyhow/struct.Error.html)
to handle errors in a more naive way.

[^encoding]: There are more general terms the problem space of formatting
and parsing a context, such as _serialization_ or _encoding_.
Instead of strings, a more general solution may use types such as _bytes_
or _buffers_. Although it is possible to design a generalized solution
for encoding in CGP, it would be too much to cover the topic in this
chapter alone. As such, we use naive strings in this chapter so that
we can focus on first understanding the basic concepts in CGP.

## Linking Multiple Providers to a Concrete Context

With the providers implemented, we can now define a concrete context like
`Person`, and link it with the given providers. However, since there are
multiple providers, we need to first define an _aggregated provider_
called `PersonComponents`, which would implement both `StringFormatter`
and `StringParser` by delegating the call to the actual providers.

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait HasComponents {
#     type Components;
# }
#
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
# }
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
# pub trait StringParser<Context> {
#     fn parse_from_string(raw: &str) -> Result<Context, Error>;
# }
#
# impl<Context> CanFormatToString for Context
# where
#     Context: HasComponents,
#     Context::Components: StringFormatter<Context>,
# {
#     fn format_to_string(&self) -> Result<String, Error> {
#         Context::Components::format_to_string(self)
#     }
# }
#
# impl<Context> CanParseFromString for Context
# where
#     Context: HasComponents,
#     Context::Components: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Context::Components::parse_from_string(raw)
#     }
# }
#
# pub struct FormatAsJsonString;
#
# impl<Context> StringFormatter<Context> for FormatAsJsonString
# where
#     Context: Serialize,
# {
#     fn format_to_string(context: &Context) -> Result<String, Error> {
#         Ok(serde_json::to_string(context)?)
#     }
# }
#
# pub struct ParseFromJsonString;
#
# impl<Context> StringParser<Context> for ParseFromJsonString
# where
#     Context: for<'a> Deserialize<'a>,
# {
#     fn parse_from_string(json_str: &str) -> Result<Context, Error> {
#         Ok(serde_json::from_str(json_str)?)
#     }
# }
#
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

pub struct PersonComponents;

impl HasComponents for Person {
    type Components = PersonComponents;
}

impl StringFormatter<Person> for PersonComponents {
    fn format_to_string(person: &Person) -> Result<String, Error> {
        FormatAsJsonString::format_to_string(person)
    }
}

impl StringParser<Person> for PersonComponents {
    fn parse_from_string(raw: &str) -> Result<Person, Error> {
        ParseFromJsonString::parse_from_string(raw)
    }
}

let person = Person { first_name: "John".into(), last_name: "Smith".into() };
let person_str = r#"{"first_name":"John","last_name":"Smith"}"#;

assert_eq!(
    person.format_to_string().unwrap(),
    person_str
);

assert_eq!(
    Person::parse_from_string(person_str).unwrap(),
    person
);
```

We first define `Person` struct with auto-derived implementations of
`Serialize` and `Deserialize`. We also auto-derive `Debug` and `Eq` for use in tests
later on.

We then define a dummy struct `PersonComponents`,
which would be used to aggregate the providers for `Person`. Compared to the
previous chapter, we implement `HasComponents` for `Person` with `PersonComponents`
as the provider.

We then implement the provider traits `StringFormatter` and `StringParser`
for `PersonComponents`, with the actual implementation forwarded to
`FormatAsJsonString` and `ParseFromJsonString`.

Inside the test that follows, we verify that the wiring indeed automatically
implements `CanFormatToString` and `CanParseFromString` for `Person`,
with the JSON implementation used.

## Blanket Provider Implementation

Although the previous example works, the boilerplate for forwarding multiple
implementations by `PersonComponents` seems a bit tedious and redundant.
The main differences between the two implementation boilerplate is that
we want to choose `FormatAsJsonString` as the provider for `StringFormatter`,
and `ParseFromJsonString` as the provider for `StringParser`.

Similar to how we can use `HasComponents` with blanket implementations to link
a consumer with a provider, we can reduce the boilerplate required by using
similar pattern to link a provider with _another_ provider:

```rust
# extern crate anyhow;
#
# use anyhow::Error;
#
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
# }
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
# pub trait StringParser<Context> {
#     fn parse_from_string(raw: &str) -> Result<Context, Error>;
# }
#
pub trait DelegateComponent<Name> {
    type Delegate;
}

pub struct StringFormatterComponent;

pub struct StringParserComponent;

impl<Context, Component> StringFormatter<Context> for Component
where
    Component: DelegateComponent<StringFormatterComponent>,
    Component::Delegate: StringFormatter<Context>,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Component::Delegate::format_to_string(context)
    }
}

impl<Context, Component> StringParser<Context> for Component
where
    Component: DelegateComponent<StringParserComponent>,
    Component::Delegate: StringParser<Context>,
{
    fn parse_from_string(raw: &str) -> Result<Context, Error> {
        Component::Delegate::parse_from_string(raw)
    }
}
```

The `DelegateComponent` is similar to the `HasComponents` trait, but it is intended
to be implemented by providers instead of concrete contexts. It also has an extra
generic `Name` type that is used to differentiate which component the provider
delegation is intended for.

To make use of the `Name` parameter, we first need to assign names to the CGP
components that we have defined. We first define the dummy struct
`StringFormatterComponent` to be used as the name for `StringFormatter`,
and `StringParserComponent` to be used as the name for `StringParser`.
In general, we can choose any type as the component name. However by convention,
we choose to add a -`Component` postfix to the name of the provider trait
to be used as the name of the component.

We then define a blanket implementation for `StringFormatter`, which is implemented
for a provider type `Component` with the following conditions: if the provider
implements `DelegateComponent<StringFormatterComponent>`, and if the associated type
`Delegate` also implements `StringFormatter<Context>`, then `Component` also
implements `StringFormatter<Context>` by delegating the implementation to `Delegate`.

Following the same pattern, we also define a blanket implementation for `StringParser`.
The main difference here is that the name `StringParserComponent` is used as the
type argument to `DelegateComponent`. In other words, different blanket provider
implementations make use of different `Name` types for `DelegateComponent`, allowing
different `Delegate` to be used depending on the `Name`.

## Using `DelegateComponent`

It may take a while to fully understand how the blanket implementations with
`DelegateComponent` and `HasComponents` work. But since the same pattern will
be used everywhere, it would hopefully become clear as we see more examples.
It would also help to see how the blanket implementation is used, by going
back to the example of implementing the concrete context `Person`.

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait HasComponents {
#     type Components;
# }
#
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
# }
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
# pub trait StringParser<Context> {
#     fn parse_from_string(raw: &str) -> Result<Context, Error>;
# }
#
# impl<Context> CanFormatToString for Context
# where
#     Context: HasComponents,
#     Context::Components: StringFormatter<Context>,
# {
#     fn format_to_string(&self) -> Result<String, Error> {
#         Context::Components::format_to_string(self)
#     }
# }
#
# impl<Context> CanParseFromString for Context
# where
#     Context: HasComponents,
#     Context::Components: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Context::Components::parse_from_string(raw)
#     }
# }
#
# pub struct FormatAsJsonString;
#
# impl<Context> StringFormatter<Context> for FormatAsJsonString
# where
#     Context: Serialize,
# {
#     fn format_to_string(context: &Context) -> Result<String, Error> {
#         Ok(serde_json::to_string(context)?)
#     }
# }
#
# pub struct ParseFromJsonString;
#
# impl<Context> StringParser<Context> for ParseFromJsonString
# where
#     Context: for<'a> Deserialize<'a>,
# {
#     fn parse_from_string(json_str: &str) -> Result<Context, Error> {
#         Ok(serde_json::from_str(json_str)?)
#     }
# }
#
# pub trait DelegateComponent<Name> {
#     type Delegate;
# }
#
# pub struct StringFormatterComponent;
#
# pub struct StringParserComponent;
#
# impl<Context, Component> StringFormatter<Context> for Component
# where
#     Component: DelegateComponent<StringFormatterComponent>,
#     Component::Delegate: StringFormatter<Context>,
# {
#     fn format_to_string(context: &Context) -> Result<String, Error> {
#         Component::Delegate::format_to_string(context)
#     }
# }
#
# impl<Context, Component> StringParser<Context> for Component
# where
#     Component: DelegateComponent<StringParserComponent>,
#     Component::Delegate: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Component::Delegate::parse_from_string(raw)
#     }
# }
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

pub struct PersonComponents;

impl HasComponents for Person {
    type Components = PersonComponents;
}

impl DelegateComponent<StringFormatterComponent> for PersonComponents {
    type Delegate = FormatAsJsonString;
}

impl DelegateComponent<StringParserComponent> for PersonComponents {
    type Delegate = ParseFromJsonString;
}

let person = Person { first_name: "John".into(), last_name: "Smith".into() };
let person_str = r#"{"first_name":"John","last_name":"Smith"}"#;

assert_eq!(
    person.format_to_string().unwrap(),
    person_str
);

assert_eq!(
    Person::parse_from_string(person_str).unwrap(),
    person
);
```

Instead of implementing the provider traits, we now only need to implement
`DelegateComponent<StringFormatterComponent>` and `DelegateComponent<StringParserComponent>`
for `PersonComponents`. Rust's trait system would then automatically make use of
the blanket implementations to implement `CanFormatToString` and `CanParseFromString`
for `Person`.

As we will see in the next chapter, we can make use of macros to further simplify
the component delegation, making it as simple as one line to implement such delegation.

## Switching Provider Implementations

With the given examples, some readers may question why is there a need to define
multiple providers for the JSON implementation, when we can just define one provide
struct and implement both provider traits for it.

The use of two providers in this chapter is mainly used as demonstration on how
to delegate and combine multiple providers. In practice, as the number of CGP
components increase, we would also quickly run into the need have multiple
provider implementations and choosing between different combination of providers.

Even with the simplified example here, we can demonstrate how a different provider
for `StringFormatter` may be needed. Supposed that we want to format the context
as prettified JSON string, we can define a separate provider `FormatAsPrettifiedJsonString`
as follows:


```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
pub struct FormatAsPrettifiedJsonString;

impl<Context> StringFormatter<Context> for FormatAsPrettifiedJsonString
where
    Context: Serialize,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(context)?)
    }
}
```

In the `StringFormatter` implementation for `FormatAsPrettifiedJsonString`,
we use `serde_json::to_string_pretty` instead of `serde_json::to_string`
to pretty format the JSON string. With CGP, both `FormatAsPrettifiedJsonString`
and `FormatAsJsonString` can co-exist peacefully, and we can easily choose
which provider to use in each concrete context implementation.


```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait HasComponents {
#     type Components;
# }
#
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
# }
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
# pub trait StringParser<Context> {
#     fn parse_from_string(raw: &str) -> Result<Context, Error>;
# }
#
# impl<Context> CanFormatToString for Context
# where
#     Context: HasComponents,
#     Context::Components: StringFormatter<Context>,
# {
#     fn format_to_string(&self) -> Result<String, Error> {
#         Context::Components::format_to_string(self)
#     }
# }
#
# impl<Context> CanParseFromString for Context
# where
#     Context: HasComponents,
#     Context::Components: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Context::Components::parse_from_string(raw)
#     }
# }
#
# pub struct FormatAsJsonString;
#
# impl<Context> StringFormatter<Context> for FormatAsJsonString
# where
#     Context: Serialize,
# {
#     fn format_to_string(context: &Context) -> Result<String, Error> {
#         Ok(serde_json::to_string(context)?)
#     }
# }
#
# pub struct FormatAsPrettifiedJsonString;
#
# impl<Context> StringFormatter<Context> for FormatAsPrettifiedJsonString
# where
#     Context: Serialize,
# {
#     fn format_to_string(context: &Context) -> Result<String, Error> {
#         Ok(serde_json::to_string_pretty(context)?)
#     }
# }
#
# pub struct ParseFromJsonString;
#
# impl<Context> StringParser<Context> for ParseFromJsonString
# where
#     Context: for<'a> Deserialize<'a>,
# {
#     fn parse_from_string(json_str: &str) -> Result<Context, Error> {
#         Ok(serde_json::from_str(json_str)?)
#     }
# }
#
# pub trait DelegateComponent<Name> {
#     type Delegate;
# }
#
# pub struct StringFormatterComponent;
#
# pub struct StringParserComponent;
#
# impl<Context, Component> StringFormatter<Context> for Component
# where
#     Component: DelegateComponent<StringFormatterComponent>,
#     Component::Delegate: StringFormatter<Context>,
# {
#     fn format_to_string(context: &Context) -> Result<String, Error> {
#         Component::Delegate::format_to_string(context)
#     }
# }
#
# impl<Context, Component> StringParser<Context> for Component
# where
#     Component: DelegateComponent<StringParserComponent>,
#     Component::Delegate: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Component::Delegate::parse_from_string(raw)
#     }
# }
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

pub struct PersonComponents;

impl HasComponents for Person {
    type Components = PersonComponents;
}

impl DelegateComponent<StringFormatterComponent> for PersonComponents {
    type Delegate = FormatAsPrettifiedJsonString;
}

impl DelegateComponent<StringParserComponent> for PersonComponents {
    type Delegate = ParseFromJsonString;
}

let person = Person { first_name: "John".into(), last_name: "Smith".into() };
let person_str = r#"{
  "first_name": "John",
  "last_name": "Smith"
}"#;

assert_eq!(
    person.format_to_string().unwrap(),
    person_str,
);

assert_eq!(
    Person::parse_from_string(person_str).unwrap(),
    person
);
```

Compared to before, the only line change is to set the `Delegate` of
`DelegateComponent<StringFormatterComponent>` to `FormatAsPrettifiedJsonString`
instead of `FormatAsJsonString`. With that, we can now easily choose between
whether to pretty print a `Person` context as JSON.

Beyond having a prettified implementation, it is also easy to think of other
kinds of generic implementations, such as using `Debug` or `Display` to format
strings, or use different encodings such as XML to format the string. With CGP,
we can define generalized component interfaces that are applicable to a wide
range of implementations. We can then make use of `DelegateComponent` to
easily choose which implementation we want to use for different concrete contexts.