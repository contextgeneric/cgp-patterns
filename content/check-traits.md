# Check Traits

By leveraging [impl-side dependencies](./impl-side-dependencies.md), CGP providers
are able to include additional dependencies that are not specified in the provider
trait. We have already seen this in action in the [previous chapter](./provider-delegation.md),
for example where the provider `FormatAsJsonString` is able to require `Context`
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

## Unsatisfied Constraint Errors

To demonstrate how such error would arise, we would reuse the same example
`PersonContext` as the [previous chapter](./component-macros.md#example-use).
Consider if we made a mistake and forgot to implement `Serialize` for `PersonContext`:

```rust
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
# extern crate cgp;
#
# use cgp::prelude::*;
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[derive_component(StringParserComponent, StringParser<Context>)]
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
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
// Note: We forgot to derive Serialize here
#[derive(Deserialize, Debug, Eq, PartialEq)]
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
```

We know that `PersonContext` uses `PersonComponents` to implement `CanFormatToString`,
and `PersonComponents` delegates the provider implementation to `FormatAsJsonString`.
However, since `FormatAsJsonString` requires `PersonContext` to implement `Serialize`,
without it `CanFormatToString` cannot be implemented on `PersonContext`.

However, notice that the above code still compiles successfully. This is because we
have not yet try to use `CanFormatToString` on person. We can try to add test code to
call `format_to_string`, and check if it works:


```rust,compile_fail
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
# extern crate cgp;
#
# use cgp::prelude::*;
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[derive_component(StringParserComponent, StringParser<Context>)]
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
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
# // Note: We forgot to derive Serialize here
# #[derive(Deserialize, Debug, Eq, PartialEq)]
# pub struct Person {
#     pub first_name: String,
#     pub last_name: String,
# }
#
# pub struct PersonComponents;
#
# impl HasComponents for Person {
#     type Components = PersonComponents;
# }
#
# delegate_components! {
#     PersonComponents {
#         StringFormatterComponent: FormatAsJsonString,
#         StringParserComponent: ParseFromJsonString,
#     }
# }
#
let person = Person { first_name: "John".into(), last_name: "Smith".into() };
println!("{}", person.format_to_string().unwrap());
```

The first time we try to call the method, our code would fail with a compile
error that looks like follows:

```text
error[E0599]: the method `format_to_string` exists for struct `Person`, but its trait bounds were not satisfied
   |
46 | pub struct Person {
   | ----------------- method `format_to_string` not found for this struct because it doesn't satisfy `Person: CanFormatToString`
...
51 | pub struct PersonComponents;
   | --------------------------- doesn't satisfy `PersonComponents: StringFormatter<Person>`
...
65 | println!("{}", person.format_to_string().unwrap());
   |                -------^^^^^^^^^^^^^^^^--
   |                |      |
   |                |      this is an associated function, not a method
   |                help: use associated function syntax instead: `Person::format_to_string()`
   |
   = note: found the following associated functions; to be used as methods, functions must have a `self` parameter
note: the candidate is defined in the trait `StringFormatter`
   |
14 |     fn format_to_string(&self) -> Result<String, Error>;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: trait bound `PersonComponents: StringFormatter<Person>` was not satisfied
   |
12 | #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: the trait `StringFormatter` must be implemented
   |
13 | / pub trait CanFormatToString {
14 | |     fn format_to_string(&self) -> Result<String, Error>;
15 | | }
   | |_^
   = help: items from traits can only be used if the trait is implemented and in scope
note: `CanFormatToString` defines an item `format_to_string`, perhaps you need to implement it
   |
13 | pub trait CanFormatToString {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: this error originates in the attribute macro `derive_component` (in Nightly builds, run with -Z macro-backtrace for more info)
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
temporary workarounds to make it easier to debug CGP errors in the meanwhile.

## Check Traits

We have learned that CGP lazily resolve dependencies and implements consumer
traits on a concrete context only when they are actively used. However,
when defining a concrete context, we would like to be able to eagerly check
that the consumer traits are implemented, so that no confusing error
should arise when the context is being used.

By convention, the best approach for implementing such checks is to define
a _check trait_, which asserts that a concrete context implements all
consumer traits that we intended to implement. the check trait would be
defined as follows:

```rust,compile_fail
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
# extern crate cgp;
#
# use cgp::prelude::*;
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[derive_component(StringParserComponent, StringParser<Context>)]
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
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
# // Note: We forgot to derive Serialize here
# #[derive(Deserialize, Debug, Eq, PartialEq)]
# pub struct Person {
#     pub first_name: String,
#     pub last_name: String,
# }
#
# pub struct PersonComponents;
#
# impl HasComponents for Person {
#     type Components = PersonComponents;
# }
#
# delegate_components! {
#     PersonComponents {
#         StringFormatterComponent: FormatAsJsonString,
#         StringParserComponent: ParseFromJsonString,
#     }
# }
#
pub trait CanUsePerson:
    CanFormatToString
    + CanParseFromString
{}

impl CanUsePerson for Person {}
```

By convention, a check trait has the name starts with `CanUse`, followed by
the name of the concrete context. We list all the consumer traits that
the concrete context should implement as the super trait. The check trait
has an empty body, followed by a blanket implementation for the target
concrete context.

In the example above, we define the check trait `CanUsePerson`, which is used
to check that the concrete context `Person` implements `CanFormatToString` and
`CanParseFromString`. If we try to compile the check trait with the
same example code as before, we would get the following error message:

```text
error[E0277]: the trait bound `FormatAsJsonString: StringFormatter<Person>` is not satisfied
   |
69 | impl CanUsePerson for Person {}
   |                       ^^^^^^ the trait `StringFormatter<Person>` is not implemented for `FormatAsJsonString`, which is required by `Person: CanFormatToString`
   |
   = help: the trait `StringFormatter<Context>` is implemented for `FormatAsJsonString`
note: required for `PersonComponents` to implement `StringFormatter<Person>`
   |
12 | #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: required for `Person` to implement `CanFormatToString`
   |
12 | #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: required by a bound in `CanUsePerson`
   |
64 | pub trait CanUsePerson:
   |           ------------ required by a bound in this trait
65 |     CanFormatToString
   |     ^^^^^^^^^^^^^^^^^ required by this bound in `CanUsePerson`
   = note: `CanUsePerson` is a "sealed trait", because to implement it you also need to implement `main::_doctest_main_check_traits_md_229_0::CanFormatToString`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
   = help: the following type implements the trait:
             Context
   = note: this error originates in the attribute macro `derive_component` (in Nightly builds, run with -Z macro-backtrace for more info)
```

The error message is still pretty confusing, but it is slightly more informative
than the previous error. Here, we can see that the top of the error says that
`StringFormatter<Person>` is not implemented for `FormatAsJsonString`.
Although it does not point to the root cause, at least we are guided to look
into the implementation of `FormatAsJsonString` to find out what went wrong there.

At the moment, there is no better way to simplify debugging further, and
we need to manually look into `FormatAsJsonString` to check why it could not
implement `StringFormatter` for the `Person` context. Here, the important
thing to look for is the additional constraints that `FormatAsJsonString`
requires on the context, which in this case is `Serialize`.

We can make use of the check trait to further track down all the indirect
dependencies of the providers that the concrete context uses. In this case,
we determine that `Serialize` is needed for `FormatAsJsonString`, and
`Deserialize` is needed for `ParseFromJsonString`. So we add them into
the super trait of `CanUsePerson` as follows:

```rust,compile_fail
# extern crate anyhow;
# extern crate serde;
# extern crate serde_json;
# extern crate cgp;
#
# use cgp::prelude::*;
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# #[derive_component(StringFormatterComponent, StringFormatter<Context>)]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[derive_component(StringParserComponent, StringParser<Context>)]
# pub trait CanParseFromString: Sized {
#     fn parse_from_string(raw: &str) -> Result<Self, Error>;
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
# // Note: We forgot to derive Serialize here
# #[derive(Deserialize, Debug, Eq, PartialEq)]
# pub struct Person {
#     pub first_name: String,
#     pub last_name: String,
# }
#
# pub struct PersonComponents;
#
# impl HasComponents for Person {
#     type Components = PersonComponents;
# }
#
# delegate_components! {
#     PersonComponents {
#         StringFormatterComponent: FormatAsJsonString,
#         StringParserComponent: ParseFromJsonString,
#     }
# }
#
pub trait CanUsePerson:
    Serialize
    + for<'a> Deserialize<'a>
    + CanFormatToString
    + CanParseFromString
{}

impl CanUsePerson for Person {}
```

When we try to compile `CanUsePerson` again, we would see a different error
message at the top:

```text
error[E0277]: the trait bound `Person: Serialize` is not satisfied
   |
71 | impl CanUsePerson for Person {}
   |                       ^^^^^^ the trait `Serialize` is not implemented for `Person`
   |
   = note: for local types consider adding `#[derive(serde::Serialize)]` to your `Person` type
   = note: for types from other crates check whether the crate offers a `serde` feature flag
```

This tells us that we have forgotten to implement `Serialize` for Person.
We can then take our action to properly fill in the missing dependencies.