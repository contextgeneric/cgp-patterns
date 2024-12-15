# Debugging Techniques

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
`Person` context as the [previous chapter](./component-macros.md#example-use).
Consider if we made a mistake and forgot to implement `Serialize` for `Person`:

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
# #[cgp_component {
#    name: StringFormatterComponent,
#    provider: StringFormatter,
#    context: Context,
# }]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[cgp_component {
#    name: StringParserComponent,
#    provider: StringParser,
#    context: Context,
# }]
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
# extern crate cgp;
#
# use cgp::prelude::*;
# use anyhow::Error;
# use serde::{Serialize, Deserialize};
#
# #[cgp_component {
#    name: StringFormatterComponent,
#    provider: StringFormatter,
#    context: Context,
# }]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[cgp_component {
#    name: StringParserComponent,
#    provider: StringParser,
#    context: Context,
# }]
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
# #[cgp_component {
#    name: StringFormatterComponent,
#    provider: StringFormatter,
#    context: Context,
# }]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[cgp_component {
#    name: StringParserComponent,
#    provider: StringParser,
#    context: Context,
# }]
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
  --> debugging-techniques.md:330:23
   |
77 | impl CanUsePerson for Person {}
   |                       ^^^^^^ the trait `StringFormatter<Person>` is not implemented for `FormatAsJsonString`, which is required by `Person: CanFormatToString`
   |
   = help: the trait `StringFormatter<Context>` is implemented for `FormatAsJsonString`
note: required for `PersonComponents` to implement `StringFormatter<Person>`
  --> debugging-techniques.md:265:1
   |
12 | / #[cgp_component {
13 | |    name: StringFormatterComponent,
14 | |    provider: StringFormatter,
15 | |    context: Context,
16 | | }]
   | |__^
note: required for `Person` to implement `CanFormatToString`
  --> debugging-techniques.md:265:1
   |
12 | / #[cgp_component {
13 | |    name: StringFormatterComponent,
14 | |    provider: StringFormatter,
15 | |    context: Context,
   | |             ^^^^^^^
16 | | }]
   | |__^
note: required by a bound in `CanUsePerson`
  --> debugging-techniques.md:326:5
   |
72 | pub trait CanUsePerson:
   |           ------------ required by a bound in this trait
73 |     CanFormatToString
   |     ^^^^^^^^^^^^^^^^^ required by this bound in `CanUsePerson`
   = note: `CanUsePerson` is a "sealed trait", because to implement it you also need to implement `main::_doctest_main_debugging_techniques_md_255_0::CanFormatToString`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
   = help: the following type implements the trait:
             Context
   = note: this error originates in the attribute macro `cgp_component` (in Nightly builds, run with -Z macro-backtrace for more info)

error: aborting due to 1 previous error
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
# #[cgp_component {
#    name: StringFormatterComponent,
#    provider: StringFormatter,
#    context: Context,
# }]
# pub trait CanFormatToString {
#     fn format_to_string(&self) -> Result<String, Error>;
# }
#
# #[cgp_component {
#    name: StringParserComponent,
#    provider: StringParser,
#    context: Context,
# }]
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

## Debugging Check Traits

Due to the need for check traits, the work for implementing a concrete context often
involves wiring up the providers, and then checking that the providers and all their
dependencies are implemented. As the number of components increase, the number of
dependencies we need to check also increase accordingly.

When encountering errors in the check traits, it often helps to comment out a large
portion of the dependencies, to focus on resolving the errors arise from a specific
dependency. For example, in the check trait we can temporarily check for the
implementation of `CanFormatToString` by commenting out all other constraints as follows:

```rust,ignore
pub trait CanUsePerson:
    Sized
    // + Serialize
    // + for<'a> Deserialize<'a>
    + CanFormatToString
    // + CanParseFromString
{}
```

We add a dummy constaint like `Sized` in the beginning of the super traits for
`CanUsePerson`, so that we can easily comment out individual lines and not worry
about whether it would lead to a dangling `+` sign.
We can then pin point the error to a specific provider, and then continue
tracing the missing dependencies from there. We would then notice that
`FormatAsJsonString` requires `Serialize`, which we can then update the
commented code to:

```rust,ignore
pub trait CanUsePerson:
    Sized
    + Serialize
    // + for<'a> Deserialize<'a>
    // + CanFormatToString
    // + CanParseFromString
{}
```

This technique can hopefully help speed up the debugging process, and determine
which dependency is missing.

## Improving The Compiler Error Message

The need of manual debugging using check traits is probably one of the major blockers
for spreading CGP for wider adoption. However, it is possible to improve the error
messages returned from the Rust compiler so that we can more easily find out what
went wrong.

When Rust fails to resolve the constraints for `Person: CanFormatString`, it in fact
knows that the reason for the failure is caused by unsatisfied _indirect_ constraints
such as `Person: Serialize`. So what we need to do is to make Rust prints out the
unsatisfied constraints.

This has been reported as [issue #134346](https://github.com/rust-lang/rust/issues/134346)
at the Rust project, with a pending fix available as
[pull request #134348](https://github.com/rust-lang/rust/pull/134348).
But until the patch is merged and stabilized, you can try to use our custom fork of the
Rust compiler to debug any CGP error.

Following are the steps to use the modified Rust compiler:

- Clone our fork of the Rust compiler at `https://github.com/contextgeneric/rust.git`, or add it as a secondary git remote.
- Checkout the branch `show-pending-constraints-in-fulfillment-error`.
- Build the Rust compiler following the [official guide](https://rustc-dev-guide.rust-lang.org/building/how-to-build-and-run.html).
- Link the compiled custom compiler using `rustup link`, aliasing it to a custom name like `cgp`. e.g. `rustup toolchain link cgp build/host/stage1`.
- Run your project using the custom compiler, e.g. `cargo +cgp check`.

If everything is working, you should see similar error messages as before, but with additional information included:

```text
error[E0277]: the trait bound `FormatAsJsonString: StringFormatter<Person>` is not satisfied
  --> content/debugging-techniques.md:330:23
   |
77 | impl CanUsePerson for Person {}
   |                       ^^^^^^ the trait `StringFormatter<Person>` is not implemented for `FormatAsJsonString`
   |
   = help: the following constraint is not satisfied: `Person: Serialize`
   = help: the trait `StringFormatter<Context>` is implemented for `FormatAsJsonString`
note: required for `PersonComponents` to implement `StringFormatter<Person>`
```

After the main error is shown, we can also see an extra help hint that says:
`the following constraint is not satisfied: Person: Serialize`. Thanks to that,
we can now much more easily pin point the source of error, and proceed to fix
our CGP program.

We hope that our patch will soon be accepted by the Rust project, so that future
versions of Rust will be more accessible for CGP. When that happens, we will
update this chapter to reflect the changes.