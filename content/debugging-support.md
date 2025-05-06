# Debugging Support

By leveraging [impl-side dependencies](./impl-side-dependencies.md), CGP providers
are able to include additional dependencies that are not specified in the provider
trait. We have already seen this in action in the [previous chapter](./provider-delegation.md), for example,
where the provider `FormatAsJsonString` is able to require `Context`
to implement `Serialize`, while that is not specified anywhere in the provider
trait `StringFormatter`.

We have also went through how provider delegation can be done using
`DelegateComponent`, which an aggregated provider like `PersonComponents`
can use to delegate the implementation of `StringFormatter` to `FormatAsJsonString`.
Within this delegation, we can also see that the requirement for `Context`
to implement `Serialize` is not required in any part of the code.

In fact, because the provider constraints are not enforced in `DelegateComponent`,
the delegation would always be successful, even if some provider constraints
are not satisfied. In other words, the impl-side provider constraints are
enforced _lazily_ in CGP, and compile-time errors would only arise when we
try to use a consumer trait against a concrete context.

## Unsatisfied Dependency Errors

To demonstrate how such error would arise, we would reuse the same example
`Person` context as the [previous chapter](./provider-delegation.md).
Consider if we made a mistake and forgot to implement `Serialize` for `Person`:


```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait HasProvider {
#     type Provider;
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
#     Context: HasProvider,
#     Context::Provider: StringFormatter<Context>,
# {
#     fn format_to_string(&self) -> Result<String, Error> {
#         Context::Provider::format_to_string(self)
#     }
# }
#
# impl<Context> CanParseFromString for Context
# where
#     Context: HasProvider,
#     Context::Provider: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Context::Provider::parse_from_string(raw)
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

impl DelegateComponent<StringParserComponent> for PersonComponents {
    type Delegate = ParseFromJsonString;
}
```

We know that `Person` uses `PersonComponents` to implement `CanFormatToString`,
and `PersonComponents` delegates the provider implementation to `FormatAsJsonString`.
However, since `FormatAsJsonString` requires `Person` to implement `Serialize`,
without it `CanFormatToString` cannot be implemented on `PersonContext`.

However, notice that the above code still compiles successfully. This is because we
have not yet try to use `CanFormatToString` on person. We can try to add test code to
call `format_to_string`, and check if it works:

```rust,compile_fail
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait HasProvider {
#     type Provider;
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
#     Context: HasProvider,
#     Context::Provider: StringFormatter<Context>,
# {
#     fn format_to_string(&self) -> Result<String, Error> {
#         Context::Provider::format_to_string(self)
#     }
# }
#
# impl<Context> CanParseFromString for Context
# where
#     Context: HasProvider,
#     Context::Provider: StringParser<Context>,
# {
#     fn parse_from_string(raw: &str) -> Result<Context, Error> {
#         Context::Provider::parse_from_string(raw)
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
# // Note: We pretend to forgot to derive Serialize here
# #[derive(Deserialize, Debug, Eq, PartialEq)]
# pub struct Person {
#     pub first_name: String,
#     pub last_name: String,
# }
#
# pub struct PersonComponents;
#
# impl HasProvider for Person {
#     type Provider = PersonComponents;
# }
#
# impl DelegateComponent<StringFormatterComponent> for PersonComponents {
#     type Delegate = FormatAsJsonString;
# }
#
# impl DelegateComponent<StringParserComponent> for PersonComponents {
#     type Delegate = ParseFromJsonString;
# }
#
let person = Person { first_name: "John".into(), last_name: "Smith".into() };
println!("{}", person.format_to_string().unwrap());
```

The first time we try to call the method, our code would fail with a compile
error that looks like follows:

```text
error[E0599]: the method `format_to_string` exists for struct `Person`, but its trait bounds were not satisfied
  --> debugging-techniques.md:180:23
   |
54 | pub struct Person {
   | ----------------- method `format_to_string` not found for this struct because it doesn't satisfy `Person: CanFormatToString`
...
59 | pub struct PersonComponents;
   | --------------------------- doesn't satisfy `PersonComponents: StringFormatter<Person>`
...
73 | println!("{}", person.format_to_string().unwrap());
   |                -------^^^^^^^^^^^^^^^^--
   |                |      |
   |                |      this is an associated function, not a method
   |                help: use associated function syntax instead: `Person::format_to_string()`
   |
   = note: found the following associated functions; to be used as methods, functions must have a `self` parameter
note: the candidate is defined in the trait `StringFormatter`
  --> debugging-techniques.md:125:5
   |
18 |     fn format_to_string(&self) -> Result<String, Error>;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: trait bound `PersonComponents: StringFormatter<Person>` was not satisfied
  --> debugging-techniques.md:119:1
   |
12 | / #[cgp_component {
13 | |    name: StringFormatterComponent,
14 | |    provider: StringFormatter,
15 | |    context: Context,
   | |             ^^^^^^^
16 | | }]
   | |__^
note: the trait `StringFormatter` must be implemented
  --> debugging-techniques.md:124:1
   |
17 | / pub trait CanFormatToString {
18 | |     fn format_to_string(&self) -> Result<String, Error>;
19 | | }
   | |_^
   = help: items from traits can only be used if the trait is implemented and in scope
note: `CanFormatToString` defines an item `format_to_string`, perhaps you need to implement it
  --> debugging-techniques.md:124:1
   |
17 | pub trait CanFormatToString {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: this error originates in the attribute macro `cgp_component` (in Nightly builds, run with -Z macro-backtrace for more info)

error: aborting due to 1 previous error
```

Unfortunately, the error message returned from Rust is very confusing, and not
helpful at all in guiding us to the root cause. For an inexperience developer,
the main takeaway from the error message is just that `CanFormatString` is
not implemented for `Person`, but the developer is left entirely on their
own to find out how to fix it.

One main reason we get such obscured errors is because the implementation of
`CanFormatString` is done through two indirect blanket implementations. As Rust
was not originally designed for blanket implementations to be used this way,
it does not follow through to explain why the blanket implementation is not
implemented.

Technically, there is no reason why the Rust compiler cannot be improved to
show more detailed errors to make using CGP easier. However, improving the
compiler will take time, and we need to present strong argument on why
such improvement is needed, e.g. through this book. Until then, we need
workarounds to make it easier to debug CGP errors in the meanwhile.

## `IsProviderFor` Trait

Since `cgp` v0.4, we have developed a technique to explicitly propagate the constraint
requirements, and "trick" Rust to show us the error messages that we need.
We will first define a `IsProviderFor` trait as follows:

```rust
pub trait IsProviderFor<Component, Context, Params = ()> {}
```

The `IsProviderFor` trait is a marker trait that can be trivially implemented.
It is intended to be implemented by provider structs, and is parameterized by 3 generic parameters:

- `Component` - The component name type that corresponds to the provider implementation.
- `Context` - The context type that is implemented by the provider.
- `Params` - Any additional generic parameters present in the provider trait, combined as a tuple.

Even though the `IsProviderFor` trait can be trivially implemented, we intentionally include
additional constraints that are exactly the same as the original constraints specified in
the corresponding provider trait implementation.

For example, the provider `FormatAsJsonString` in the earlier example would implement
`IsProviderFor` as follows:

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait IsProviderFor<Component, Context, Params = ()> {}
#
# pub struct StringFormatterComponent;
#
# pub trait StringFormatter<Context> {
#     fn format_to_string(context: &Context) -> Result<String, Error>;
# }
#
pub struct FormatAsJsonString;

impl<Context> StringFormatter<Context> for FormatAsJsonString
where
    Context: Serialize,
{
    fn format_to_string(context: &Context) -> Result<String, Error> {
        Ok(serde_json::to_string(context)?)
    }
}

impl<Context> IsProviderFor<StringFormatterComponent, Context> for FormatAsJsonString
where
    Context: Serialize,
{ }
```

The way to understand the trait implementation is follows: `FormatAsJsonString`
has a provider implementation for `StringFormatterComponent` with the context `Context`,
given that `Context: Serialize`.

We can think of `IsProviderFor` trait to act as a "carrier" for the hidden constraints
of provider traits. With it, instead of trying to check each provider trait implementation,
we only need to check for the implementation of one trait, `IsProviderFor`.

## Propagating `IsProviderFor` Constraints

Now that we have captured our constraints using `IsProviderFor`, we need to somehow propagate the
constraint upward through the delegation chain, so that the context provider `PersonComponents`
also implements `IsProviderFor` with the same constraints as its delegate, `FormatAsJsonString`.

We would do that by modifying the provider trait definition, so that `IsProviderFor` becomes a
_supertrait_ of the provider trait:

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# pub trait IsProviderFor<Component, Context, Params = ()> {}
#
# pub trait DelegateComponent<Name> {
#     type Delegate;
# }
#
# pub struct StringFormatterComponent;
#
pub trait StringFormatter<Context>: IsProviderFor<StringFormatterComponent, Context> {
    fn format_to_string(context: &Context) -> Result<String, Error>;
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
```

We make it a requirement that in order for a provider to implement `StringFormatter<Context>`, the provider
also needs to implement `IsProviderFor<StringFormatterComponent, Context>`.
The supertrait constraint not only makes sure that we don't forget to always implement `IsProviderFor`,
but also triggers Rust to show any unsatisfied constraints that were hidden previously.

Additionally, we also modify the blanket implementation of `StringFormatter`, so that when a provider like
`PersonComponents` delegates the implementation, it would need to explicitly implement
`IsProviderFor<StringFormatterComponent, Context>` in addition to implementing `DelegateComponent<StringFormatterComponent>`.

With the new requirements in place, when we delegate `PersonComponents`'s provider implementation of `StringFormatterComponent`,
we would also write an implementation of `IsProviderFor` for it as follows:

```rust,ignore
impl DelegateComponent<StringFormatterComponent> for PersonComponents {
    type Delegate = FormatAsJsonString;
}

impl<Context> IsProviderFor<StringFormatterComponent, Context>
   for PersonComponents
where
    FormatAsJsonString: IsProviderFor<StringFormatterComponent, Context>
{
}
```

Notice here that when implementing `IsProviderFor` for `PersonComponents`, we add a constraint that
directly requires `FormatAsJsonString` to also implement `IsProviderFor` with the same generic parameters.
By doing so, we resurface the constraints to Rust, so that it would recursively look into the
`IsProviderFor` trait bounds, and print out any unsatisfied constraints in the error messages.

At this point, you may wonder why not link the provider trait directly within the delegated implementation of `IsProviderFor`, such as:

```rust,ignore
impl<Context> IsProviderFor<StringFormatterComponent, Context>
   for PersonComponents
where
    FormatAsJsonString: StringFormatter<Context>,
{
}
```

The main reason to _not_ do this is that it requires direct access to the provider trait, which is not as simple dealing with simple types. Furthermore, the provider trait may contain additional where clauses, which would also need to be propagated explicitly.

By making use of `IsProviderFor`, we are essentially "erasing" everything at the trait level, and use a single trait to represent all other provider traits. After all, the only thing that we are interested here is to propagate the constraints for the purpose of showing better error messages.

## Check Traits

Now that we have the wirings for `IsProviderFor` in place, we can implement _check traits_ to check
on whether the provider traits that we want to use with `Person` are implemented by its provider,
`PersonComponents`.

```rust,ignore
pub trait CanUsePersonComponents:
    IsProviderFor<StringFormatterComponent, Person>
    + IsProviderFor<StringParserComponent, Person>
{
}

impl CanUsePersonComponents for PersonComponents {}
```

To put it simply, a check trait is defined just for checking whether a type implements other traits that we are
interested to use with that type. For our case, we want to check that
`PersonComponents: IsProviderFor<StringFormatterComponent, Person>`
and `IsProviderFor<StringParserComponent, Person>`, since we expect `PersonComponents` to implement
`StringFormatter<Person>` and `StringParser<Person>`. So we define a check trait called `CanUsePersonComponents`,
and put the `IsProviderFor` constraints as the supertrait.

The check trait is then followed by an implementation of `CanUsePersonComponents` for `PersonComponents`,
which acts as an _assertion_ that `PersonComponents` implements all the supertraits that we specified.

With the checks in place, we now get a compile error message that shows us exactly what we need to fix,
which is to implement `Serialize` for `Person`:

```text
error[E0277]: the trait bound `Person: Serialize` is not satisfied
   --> src/lib.rs:155:33
    |
155 | impl CanUsePersonComponents for PersonComponents {}
    |                                 ^^^^^^^^^^^^^^^^ the trait `Serialize` is not implemented for `Person`
    |
    = note: for local types consider adding `#[derive(serde::Serialize)]` to your `Person` type
    = note: for types from other crates check whether the crate offers a `serde` feature flag
    = help: the following other types implement trait `Serialize`:
              &'a T
              &'a mut T
              ()
              (T,)
              (T0, T1)
              (T0, T1, T2)
              (T0, T1, T2, T3)
              (T0, T1, T2, T3, T4)
            and 131 others
note: required for `FormatAsJsonString` to implement `IsProviderFor<StringFormatterComponent, Person>`
   --> src/lib.rs:91:15
    |
91  | impl<Context> IsProviderFor<StringFormatterComponent, Context>
    |               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
92  |     for FormatAsJsonString
    |         ^^^^^^^^^^^^^^^^^^
93  | where
94  |     Context: Serialize,
    |              --------- unsatisfied trait bound introduced here
    = note: 1 redundant requirement hidden
    = note: required for `PersonComponents` to implement `IsProviderFor<StringFormatterComponent, Person>`
note: required by a bound in `CanUsePersonComponents`
   --> src/lib.rs:150:5
    |
149 | pub trait CanUsePersonComponents:
    |           ---------------------- required by a bound in this trait
150 |     IsProviderFor<StringFormatterComponent, Person>
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `CanUsePersonComponents`
```

As we can see, the use of `IsProviderFor` helps make it possible for us to debug our CGP programs again.

## `CanUseComponent` Trait

Although we can directly check for the implementation of `IsProviderFor`, the definition is not as straightforward as
defining checks directly with the `Person` context. Furthermore, the context type `Person` needs to be repeatedly
specified for each supertrait, which becomes tedious and adds noise.

We can improve the check definitions by introducing a helper `CanUseComponent` trait that is defined as follows:

```rust
# pub trait HasProvider {
#     type Provider;
# }
#
pub trait IsProviderFor<Component, Context, Params = ()> {}

pub trait CanUseComponent<Component, Params = ()> {}

impl<Context, Component, Params> CanUseComponent<Component, Params> for Context
where
    Context: HasProvider,
    Context::Provider: IsProviderFor<Component, Context, Params>,
{
}
```

Instead of being implemented by a provider, `CanUseComponent` is intended to be implemented by a context type that
implements `HasProvider`. Compared to `IsProviderFor`, there is no need to specify an explicit `Context` parameter.

`CanUseComponent` also has a blanket implementation, making it automatically implemented for any `Context` type
that implements `HasProvider`, with `Context::Provider` implementing `IsProviderFor`.

In other words, we are using `CanUseComponent` as a _trait alias_ to `IsProviderFor`, with improved ergonomics
that we can check directly against a context type.

Using `CanUseComponent`, we can re-define our check trait much simply as follows:

```rust,ignore
pub trait CanUsePerson:
    CanUseComponent<StringFormatterComponent>
    + CanUseComponent<StringParserComponent>
{
}

impl CanUsePerson for Person {}
```

Compared to before, we define the check trait `CanUsePerson` directly on `Person` instead of `PersonComponents`.
When we try compiling, we would still get the same error message that shows us the required information
to fix the error:

```text
error[E0277]: the trait bound `Person: CanUseComponent<StringFormatterComponent>` is not satisfied
   --> src/lib.rs:164:23
    |
164 | impl CanUsePerson for Person {}
    |                       ^^^^^^ the trait `Serialize` is not implemented for `Person`
...
```

## Limitations

The use of `IsProviderFor` significantly improves the developer experience for CGP, which was significantly harder
to debug prior to the introduction of this technique in v0.4.0.

At this point, you might be concerned of the additional boilerplate required to implement and propagate the constraints for `IsProviderFor`. However, as we will see in the [next chapter](./component-macros.md), the CGP macros will automate the bulk of the implementation of `IsProviderFor`, and only require lightweight attribute macros to be applied to enable the code generation.

It is worth noting that the `IsProviderFor` trait is introduced as a workaround to improve the error messages of CGP code in Rust. Hypothetically, if Rust could provide better support of showing the relevant error messages, we could entirely remove the use of `IsProviderFor` in future versions of CGP.

On the other hand, the use of `IsProviderFor` can serve as a great showcase to the Rust compiler team on what error messages should have been shown by default, and make it easier to evaluate what should be the official fix in the Rust compiler.

That said, the error messages shown via `IsProviderFor` can sometimes be too verbose, especially when an application contains many providers with deeply nested dependencies. The reason is that any unsatisfied constraint from deeply nested dependencies can propagate all the way up through chains of `IsProviderFor` implementations. Although this can help us pinpoint the root cause, it could generate too much noise when showing all errors from the intermediary layers.

When encountering heap of error messages generated from `IsProviderFor`, a useful tip is that the relevant error message may be hiding near the bottom. So it may be useful to read from the bottom up instead of top down.

## Interactive Debugging with Argus

A potential improvement that we are currently exploring is to make use of [Argus](https://cel.cs.brown.edu/paper/an-interactive-debugger-for-rust-trait-errors/) to help navigate error messages generated from CGP code. Argus provides an interactive debugger for trait-related errors, which may be well suited to be used for debugging CGP code.

We will add further details in the future to share potential integration with Argus. For now, interested readers are encouraged to check out the project, and perhaps make contribution to make such integration possible.

## Conclusion

In this chapter, we have learned how CGP makes use of the `IsProviderFor` trait to help show relevant error messages when encountering unsatisfied constraints. In the next chapter, we will walk through how to automate the generation of all the relevant boilerplates that we have learned so far, and write succint CGP code using macros.

We will show the full example that we have walked through earlier, with the addition of `IsProviderFor` into the code:

```rust,compile_fail
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
#
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
```