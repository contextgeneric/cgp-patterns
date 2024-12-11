# Component Macros

At this point, we have covered all basic building blocks of defining CGP components.
In summary, a CGP component is consist of the following building blocks:

- A consumer trait.
- A provider trait.
- A component name type.
- A blanket implementation of the consumer trait using `HasComponents`.
- A blanket implementation of the provider trait using `DelegateComponent`.

Syntactically, all CGP components follow the same pattern. The pattern is
roughly as follows:

```rust,ignore
// Consumer trait
pub trait CanPerformAction<GenericA, GenericB, ...>:
    ConstraintA + ConstraintB + ...
{
    fn perform_action(
        &self,
        arg_a: ArgA,
        arg_b: ArgB,
        ...
    ) -> Output;
}

// Provider trait
pub trait ActionPerformer<Context, GenericA, GenericB, ...>
where
    Context: ConstraintA + ConstraintB + ...,
{
    fn perform_action(
        context: &Context,
        arg_a: ArgA,
        arg_b: ArgB,
        ...
    ) -> Output;
}

// Component name
pub struct ActionPerformerComponent;

// Blanket implementation for consumer trait
impl<Context, GenericA, GenericB, ...>
    CanPerformAction<GenericA, GenericB, ...> for Context
where
    Context: HasComponents + ConstraintA + ConstraintB + ...,
    Context::Components: ActionPerformer<Context>,
{
    fn perform_action(
        &self,
        arg_a: ArgA,
        arg_b: ArgB,
        ...
    ) -> Output {
        Context::Components::perform_action(self, arg_a, arg_b, ...)
    }
}

// Blanket implementation for provider trait
impl<Context, Component, GenericA, GenericB, ...>
    ActionPerformer<Context, GenericA, GenericB, ...>
    for Component
where
    Context: ConstraintA + ConstraintB + ...,
    Component: DelegateComponent<ActionPerformerComponent>,
    Component::Delegate: ActionPerformer<Context, GenericA, GenericB, ...>,
{
    fn perform_action(
        context: &Context,
        arg_a: ArgA,
        arg_b: ArgB,
        ...
    ) -> Output {
        Component::Delegate::perform_action(context, arg_a, arg_b, ...)
    }
}
```

## `cgp_component` Macro

With the repetitive pattern, it makes sense that we should be able to
just define the consumer trait, and make use of Rust macros to generate
the remaining code. The author has published the [`cgp`](https://docs.rs/cgp)
Rust crate that provides the `cgp_component` attribute macro that can be used for
this purpose. Using the macro, the same code as above can be significantly
simplified to the following:

```rust,ignore
use cgp::prelude::*;

#[cgp_component {
    name: ActionPerformerComponent,
    provider: ActionPerformer,
    context: Context,
}]
pub trait CanPerformAction<GenericA, GenericB, ...>:
    ConstraintA + ConstraintB + ...
{
    fn perform_action(
        &self,
        arg_a: ArgA,
        arg_b: ArgB,
        ...
    ) -> Output;
}
```

To use the macro, the bulk import statement `use cgp::prelude::*` has to
be used to bring all CGP constructs into scope. This includes the
`HasComponents` and `DelegateComponent` traits, which are also provided
by the `cgp` crate.

We then use `derive_component` as an attribute proc macro, with several
key-value arguments given. The `name` field is used to define the component
name type, which is called `ActionPerformerComponent`. The `provider`
field `ActionPerformer` is used for the name for the provider trait.
The `context` field `Context` is used for the generic type name of the
context when used inside the provider trait.

## `delegate_components` Macro

In addition to the `derive_component` macro, `cgp` also provides the
`delegate_components!` macro that can be used to automatically implement
`DelegateComponent` for a provider type. The syntax is roughly as follows:

```rust,ignore
use cgp::prelude::*;

pub struct TargetProvider;

delegate_components! {
    TargetProvider {
        ComponentA: ProviderA,
        ComponentB: ProviderB,
        [
            ComponentC1,
            ComponentC2,
            ...
        ]: ProviderC,
    }
}
```

The above code would be desugared into the following:

```rust,ignore
impl DelegateComponent<ComponentA> for TargetProvider {
    type Delegate = ProviderA;
}

impl DelegateComponent<ComponentB> for TargetProvider {
    type Delegate = ProviderB;
}

impl DelegateComponent<ComponentC1> for TargetProvider {
    type Delegate = ProviderC;
}

impl DelegateComponent<ComponentC2> for TargetProvider {
    type Delegate = ProviderC;
}
```

The `delegate_components!` macro accepts an argument to an existing type,
`TargetProvider`, which is expected to be defined outside of the macro.
It is followed by an open brace, and contain entries that look like
key-value pairs. For a key-value pair `ComponentA: ProviderA`, the type
`ComponentA` is used as the component name, and `ProviderA` refers to
the provider implementation.
When multiple keys map to the same value, i.e. multiple components are
delegated to the same provider implementation, the array syntax can be
used to further simplify the mapping.

## Example Use

To illustrate how `derive_component` and `delegate_components` can be
used, we revisit the code for `CanFormatToString`, `CanParseFromString`,
and `PersonContext` from the [previous chapter](./provider-delegation.md),
and look at how the macros can simplify the same code.

Following is the full code after simplification using `cgp`:

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
# extern crate cgp;
#
use cgp::prelude::*;
use anyhow::Error;
use serde::{Serialize, Deserialize};

// Component definitions

#[cgp_component {
    name: StringFormatterComponent,
    provider: StringFormatter,
    context: Context,
}]
pub trait CanFormatToString {
    fn format_to_string(&self) -> Result<String, Error>;
}

#[cgp_component {
    name: StringParserComponent,
    provider: StringParser,
    context: Context,
}]
pub trait CanParseFromString: Sized {
    fn parse_from_string(raw: &str) -> Result<Self, Error>;
}

// Provider implementations

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

// Concrete context and wiring

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

pub struct PersonComponents;

impl HasComponents for Person {
    type Components = PersonComponents;
}

delegate_components! {
    PersonComponents {
        StringFormatterComponent: FormatAsJsonString,
        StringParserComponent: ParseFromJsonString,
    }
}

# let person = Person { first_name: "John".into(), last_name: "Smith".into() };
# let person_str = r#"{"first_name":"John","last_name":"Smith"}"#;
#
# assert_eq!(
#     person.format_to_string().unwrap(),
#     person_str
# );
#
# assert_eq!(
#     Person::parse_from_string(person_str).unwrap(),
#     person
# );
```

As we can see, the new code is significantly simpler and more readable than before.
Using `derive_component`, we no longer need to explicitly define the provider
traits `StringFormatter` and `StringParser`, and the blanket implementations
can be omitted. We also make use of `delegate_components!` on `PersonComponents`
to delegate `StringFormatterComponent` to `FormatAsJsonString`, and
`StringParserComponent` to `ParseFromJsonString`.

## CGP Macros as Language Extension

The use of `cgp` crate with its macros is essential in enabling the full power
of context-generic programming in Rust. Without it, programming with CGP would
become too verbose and full of boilerplate code.

On the other hand, the use of `cgp` macros makes CGP code look much more like
programming in a _domain-specific language_ (DSL) than in regular Rust.
In fact, one could argue that CGP acts as a _language extension_ to the base
language Rust, and almost turn into its own programming language.

In a way, implementing CGP in Rust is slightly similar to implementing
OOP in C. We could think of context-generic programming being as
foundational as object-oriented programming, and may be integrated as
a core language feature in future programming languages.

Perhaps one day, there might be an equivalent of C++ to replace CGP-on-Rust.
Or perhaps more ideally, the core constructs of CGP would one day directly
supported as a core language feature in Rust.
But until that happens, the `cgp` crate serves as an experimental ground on
how context-generic programming can be done in Rust, and how it can help
build better Rust applications.

In the chapters that follow, we will make heavy use of `cgp` and its
macros to dive further into the world of context-generic programming.