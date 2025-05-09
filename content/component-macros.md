# Component Macros

At this point, we have covered all basic building blocks of defining CGP components.
In summary, a CGP component is consist of the following building blocks:

- A consumer trait.
- A provider trait.
- A component name type.
- A blanket implementation of the consumer trait using `HasCgpProvider`.
- A blanket implementation of the provider trait using `DelegateComponent`.

Syntactically, all CGP components follow the same pattern. The pattern is
roughly as follows:

```rust,ignore
// Component name
pub struct ActionPerformerComponent;

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
pub trait ActionPerformer<Context, GenericA, GenericB, ...>:
    IsProviderFor<ActionPerformerComponent, Context, (GenericA, GenericB, ...)>
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

// Blanket implementation for consumer trait
impl<Context, GenericA, GenericB, ...>
    CanPerformAction<GenericA, GenericB, ...> for Context
where
    Context: HasCgpProvider + ConstraintA + ConstraintB + ...,
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
    Component: DelegateComponent<ActionPerformerComponent>
        + IsProviderFor<ActionPerformerComponent, Context, (GenericA, GenericB, ...)>,
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

## `#[cgp_component]` Macro

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
`HasCgpProvider` and `DelegateComponent` traits, which are also provided
by the `cgp` crate.

We then use `cgp_component` as an attribute proc macro, with several
key-value arguments given. The `name` field is used to define the component
name type, which is called `ActionPerformerComponent`. The `provider`
field `ActionPerformer` is used for the name for the provider trait.
The `context` field `Context` is used for the generic type name of the
context when used inside the provider trait.

The `cgp_component` macro allows the `name` and `context` field to
be omited. When omitted, the `context` field will default to `Context`,
and the `name` field will default to `{provider}Component`.
So the same example above could be simplified to:

```rust,ignore
#[cgp_component {
    provider: ActionPerformer,
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

When only the provider name is specified, we can also omit the `key: value` syntax, and specify only the provider name:

```rust,ignore
#[cgp_component(ActionPerformer)]
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


## `delegate_components!` Macro

In addition to the `cgp_component` macro, `cgp` also provides the
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

impl<Context, Params> IsProviderFor<ComponentA, Context, Params>
    for TargetProvider
where
    ProviderA: IsProviderFor<ComponentA, Context, Params>,
{
}

impl DelegateComponent<ComponentB> for TargetProvider {
    type Delegate = ProviderB;
}

impl<Context, Params> IsProviderFor<ComponentB, Context, Params>
    for TargetProvider
where
    ProviderB: IsProviderFor<ComponentB, Context, Params>,
{
}

impl DelegateComponent<ComponentC1> for TargetProvider {
    type Delegate = ProviderC;
}

impl<Context, Params> IsProviderFor<ComponentC1, Context, Params>
    for TargetProvider
where
    ProviderC: IsProviderFor<ComponentC1, Context, Params>,
{
}

impl DelegateComponent<ComponentC2> for TargetProvider {
    type Delegate = ProviderC;
}

impl<Context, Params> IsProviderFor<ComponentC2, Context, Params>
    for TargetProvider
where
    ProviderC: IsProviderFor<ComponentC2, Context, Params>,
{
}
```

The `delegate_components!` macro accepts an argument to an existing type,
`TargetProvider`, which is expected to be defined outside of the macro.
It is followed by an open brace, and contain entries that look like
key-value pairs.

For a key-value pair `ComponentA: ProviderA`, the type `ComponentA` is used as the component name, and `ProviderA` refers to the provider implementation.
When multiple keys map to the same value, i.e. multiple components are
delegated to the same provider implementation, the array syntax can be
used to further simplify the mapping.

Instead of defining the provider struct on our own, we can also instruct `delegate_components!` to also define the provider struct for us, by adding a `new` keyword in front:

```rust,ignore
delegate_components! {
    new TargetProvider {
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

## `#[cgp_context]` Macro

The `#[cgp_context]` macro can be applied on a context struct, to automatically define the provider struct for the context and implement `HasCgpProvider` for the context.

Given the following context definition:

```rust,ignore
#[cgp_context(MyContextComponents)]
pub struct MyContext {
    ...
}
```

The macro will generate the following constructs:

```rust,ignore
pub struct MyContextComponents;

impl HasCgpProvider for MyContext {
    type CgpProvider = MyContextComponents;
}
```

If the context provider name follows the pattern `{ContextName}Components`, then the macro attribute argument can be omitted, and the code can be simplified to:

```rust,ignore
#[cgp_context]
pub struct MyContext {
    ...
}
```

## `#[cgp_provider]` Macro

When implementing a provider, the `#[cgp_provider]` macro needs to be used to automatically implement the `IsProviderFor` implementation, with all constraints within the `impl` block copied over.

Given a provider trait implementation with the pattern:

```rust,ignore
pub struct Provider;

#[cgp_provider(ActionPerformerComponent)]
impl<Context, GenericA, GenericB, ...>
    ActionPerformer<Context, GenericA, GenericB, ...>
    for Provider
where
    Context: ConstraintA + ConstraintB + ...,
    Context::Assoc: ConstraintC + ConstraintD + ...,
{ ... }
```

`#[cgp_provider]` would generate an `IsProviderFor` implementation that follows the pattern:

```rust,ignore
impl<Context, GenericA, GenericB, ...>
    IsProviderFor<ActionPerformerComponent, Context, GenericA, GenericB, ...>
    for Provider
where
    Context: ConstraintA + ConstraintB + ...,
    Context::Assoc: ConstraintC + ConstraintD + ...,
{ }
```

If the component name for the provider trait follows the format `"{ProviderTraitName}Component"`, then the component name can be omitted in the attribute argument for `#[cgp_provider]`, simplifying the code to:

```rust,ignore
pub struct Provider;

#[cgp_provider]
impl<Context, GenericA, GenericB, ...>
    ActionPerformer<Context, GenericA, GenericB, ...>
    for Provider
where
    Context: ConstraintA + ConstraintB + ...,
    Context::Assoc: ConstraintC + ConstraintD + ...,
{ ... }
```

Note, however, that the generated code would require the component type `"{ProviderTraitName}Component"` to be imported into scope. If the component name is not specified, IDEs like Rust Analyzer may not provide quick fix for auto importing the component. As a result, it may still be preferrable to include the component name attribute, especially when writing new code.

There is also a variant of the macro, `#[cgp_new_provider]`, which would also automatically define the struct for the provider. With that, the code can be defined with the `struct` definition omitted:

```rust,ignore
#[cgp_new_provider]
impl<Context, GenericA, GenericB, ...>
    ActionPerformer<Context, GenericA, GenericB, ...>
    for Provider
where
    Context: ConstraintA + ConstraintB + ...,
    Context::Assoc: ConstraintC + ConstraintD + ...,
{ ... }
```

`#[cgp_new_provider]` is mainly useful in cases where a provider only implements one provider trait. When definining a provider with multiple provider trait implementations, it may be more clear to define the provider struct explicitly, or only use `#[cgp_new_provider]` for the first `impl` block of the provider.

## `check_components!` Macro

To help with debugging CGP code, the `check_components!` macro is provided to allow us to quickly write compile-time tests on the component wiring.

Given the following code pattern:

```rust,ignore
check_components! {
    CanUseContext for Context {
        ComponentA,
        ComponentB,
        ComponentC: GenericA,
        [
            ComponentD,
            ComponentE,
        ]: [
            (GenericB1, GenericB2, ...),
            (GenericC1, GenericC2, ...),
        ],
    }
}
```

The following check trait would be generated:

```rust,ignore
pub trait CanUseContext:
    CanUseComponent<ComponentA>
    + CanUseComponent<ComponentB>
    + CanUseComponent<ComponentC, GenericA>
    + CanUseComponent<ComponentD, (GenericB1, GenericB2, ...)>
    + CanUseComponent<ComponentD, (GenericC1, GenericC2, ...)>
    + CanUseComponent<ComponentE, (GenericB1, GenericB2, ...)>
    + CanUseComponent<ComponentE, (GenericC1, GenericC2, ...)>
{}

impl CanUseContext for Context {}
```

The `check_components!` macro allows the use of array syntax at either the key or value position, when there are multiple components that share the same set of generic parameters.

## `delegate_and_check_components!` Macro

The `delegate_and_check_components!` macro combines both calls to `delegate_components!` and `check_components!`, so that wiring checks are done as soon as a delegate entry is added. This can simplify the boilerplate required to duplicate the code of listing all components in both delegate and check entries.

Given the following:

```rust,ignore
delegate_and_check_components! {
    CanUseContext for Context;
    ContextComponents {
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

The macro would expand into the equivalent of:

```rust,ignore
delegate_components! {
    ContextComponents {
        ComponentA: ProviderA,
        ComponentB: ProviderB,
        [
            ComponentC1,
            ComponentC2,
            ...
        ]: ProviderC,
    }
}

check_components! {
    CanUseContext for Context {
        ComponentA,
        ComponentB,
        ComponentC1,
        ComponentC2,
    }
}
```

You may wonder why we need define a separate macro, instead of always checking the wiring directly inside `delegate_components!`. The main reason is that while `delegate_and_check_components!` can work for the simple cases, it is more limited and cannot handle well on advanced cases where the CGP traits contain additional generic parameters. For such cases, it is still better to call `delegate_components!` and `check_components!` separately.


## Example Use

To illustrate how `cgp_component` and `delegate_components` can be
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

#[cgp_component(StringFormatter)]
pub trait CanFormatToString {
    fn format_to_string(&self) -> Result<String, Error>;
}

#[cgp_component(StringParser)]
pub trait CanParseFromString: Sized {
    fn parse_from_string(raw: &str) -> Result<Self, Error>;
}

// Provider implementations

#[cgp_new_provider]
impl<Context> StringFormatter<Context> for FormatAsJsonString
where
    Context: Serialize,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Ok(serde_json::to_string(context)?)
    }
}

#[cgp_new_provider]
impl<Context> StringParser<Context> for ParseFromJsonString
where
    Context: for<'a> Deserialize<'a>,
{
    fn parse_from_string(json_str: &str) -> Result<Context, Error> {
        Ok(serde_json::from_str(json_str)?)
    }
}

// Concrete context and wiring
#[cgp_context]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

delegate_and_check_components! {
    CanUsePerson for Person;
    PersonComponents {
        StringFormatterComponent:
            FormatAsJsonString,
        StringParserComponent:
            ParseFromJsonString,
    }
}
```

As we can see, the new code is significantly simpler and more readable than before.
Using `#[cgp_component]`, we no longer need to explicitly define the provider traits `StringFormatter` and `StringParser`, and the blanket implementations can be omitted.

With `#[cgp_new_provider]`, the `IsProviderFor` implementations for `FormatAsJsonString` and `ParseFromJsonString` are automatically implemented, together with the struct definitions.

We also make use of `delegate_and_check_components!` on `PersonComponents` to delegate `StringFormatterComponent` to `FormatAsJsonString`, and `StringParserComponent` to `ParseFromJsonString`, and then check to ensure that the wirings are implemented correctly for the `Person` context.

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
