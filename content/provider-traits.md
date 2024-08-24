# Provider Traits

In the previous chapters on [blanket implementations](./blanket-implementations.md)
and [impl-side dependencies](./impl-side-dependencies.md), we learned about the power
of using blanket `impl` blocks to simplify and hide the dependencies required by
each part of the implementation. However, one major limitation of blanket implementations
is that there cannot be multiple potentially overlapping implementations, due to
restrictions in Rust's trait system. In CGP, we can overcome this limitation by introducing
the concept of _provider traits_.

The main idea behind provider traits is to define Rust traits that are dedicated for
[providers](./provider.md) to define new implementations, and separate it from the
_consumer traits_ that are more suitable for [consumers](./consumer.md) that use the traits.
Consider a simple consumer trait `CanFormatToString`, which allows formatting a context into string:

```rust
pub trait CanFormatString {
    fn format_string(&self) -> String;
}
```

The trait we defined here is almost identical to the standard library's
[`ToString`](https://doc.rust-lang.org/std/string/trait.ToString.html) trait.
But we will duplicate the trait here to tweak how it is implemented. We first
note that the original `ToString` trait has a blanket implementation for any
type that implements `Display`:

```rust
use core::fmt::Display;
#
# pub trait CanFormatString {
#     fn format_string(&self) -> String;
# }

impl<Context> CanFormatString for Context
where
    Context: Display,
{
    fn format_string(&self) -> String {
        format!("{}", self)
    }
}
```

Although having this blanket implementation is convenient, it restricts us from
being able to format the context in other ways, such as using `Debug`.

```rust,compile_fail
use core::fmt::{Display, Debug};
#
# pub trait CanFormatString {
#     fn format_string(&self) -> String;
# }

impl<Context> CanFormatString for Context
where
    Context: Display,
{
    fn format_string(&self) -> String {
        format!("{}", self)
    }
}

// Error: conflicting implementation
impl<Context> CanFormatString for Context
where
    Context: Debug,
{
    fn format_string(&self) -> String {
        format!("{:?}", self)
    }
}
```

To overcome this limitation, we can introduce a _provider trait_ that we'd call
`StringFormatter`, which we will then use for defining implementations:

```rust
pub trait StringFormatter<Context> {
    fn format_string(context: &Context) -> String;
}
```

Compared to `CanFormatString`, the trait `StringFormatter` replaces the _implicit_
context type `Self` with an _explicit_ context type `Context`, as defined in its
type parameter. Following that, it replaces all occurrances  of `&self`
with `context: &Context`.

By avoiding the use of `Self` in provider traits, we can bypass the restrictions of
Rust trait system, and have multiple implementations defined. Continuing the earlier
example, we can define the `Display` and `Debug` implementations of `CanFormatString`
as two separate providers of `StringFormatter`:

```rust
use core::fmt::{Display, Debug};
#
# pub trait StringFormatter<Context> {
#     fn format_string(context: &Context) -> String;
# }

pub struct FormatStringWithDisplay;

pub struct FormatStringWithDebug;

impl<Context> StringFormatter<Context> for FormatStringWithDisplay
where
    Context: Display,
{
    fn format_string(context: &Context) -> String {
        format!("{}", context)
    }
}

impl<Context> StringFormatter<Context> for FormatStringWithDebug
where
    Context: Debug,
{
    fn format_string(context: &Context) -> String {
        format!("{:?}", context)
    }
}
```

With provider traits, we now have two _named_ providers `FormatStringWithDisplay`
and `FormatStringWithDebug`, which are defined as dummy structs. These structs
are not meant to be used inside any code during run time. Rather, they are used
as _identifiers_ at the _type level_ for us to refer to the providers during
compile time.

Notice that inside the implementation of `StringFormatter`, the types
`FormatStringWithDisplay` and `FormatStringWithDebug` are in the position that is
typically used for `Self`, but we don't use `Self` anywhere in the implementation.
Instead, the original `Self` type is now referred explicitly as the `Context` type,
and we use `&context` instead of `&self` inside the implementation.

From the point of view of Rust's trait system, the rules for overlapping implementation
only applies to the `Self` type. But because we have two distinct `Self` types here
(`FormatStringWithDisplay` and `FormatStringWithDebug`), the two implementations are not
considered overlapping, and we are able to define them without any compilation error.

## Using Provider Traits Directly

Although provider traits allow us to define overlapping implementations, the main downside
is that consumer code cannot make use of an implementation without explicitly choosing the
implementation.

Consider the following `Person` context defined:

```rust
use core::fmt::{self, Display, Debug};
#
# pub trait StringFormatter<Context> {
#     fn format_string(context: &Context) -> String;
# }
#
# pub struct FormatStringWithDisplay;
#
# pub struct FormatStringWithDebug;
#
# impl<Context> StringFormatter<Context> for FormatStringWithDisplay
# where
#     Context: Display,
# {
#     fn format_string(context: &Context) -> String {
#         format!("{}", context)
#     }
# }
#
# impl<Context> StringFormatter<Context> for FormatStringWithDebug
# where
#     Context: Debug,
# {
#     fn format_string(context: &Context) -> String {
#         format!("{:?}", context)
#     }
# }

#[derive(Debug)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

impl Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.first_name, self.last_name)
    }
}

let person = Person { first_name: "John".into(), last_name: "Smith".into() };

assert_eq!(
    FormatStringWithDisplay::format_string(&person),
    "John Smith"
);

assert_eq!(
    FormatStringWithDebug::format_string(&person),
    "Person { first_name: \"John\", last_name: \"Smith\" }"
);
```

Our `Person` struct is defined with both `Debug` and `Display` implementations.
When using `format_string` on a value `person: Person`, we cannot just call
`person.format_string()`. Instead, we have to explicitly pick a provider `Provider`,
and call it with `Provider::format_string(&person)`.
On the other hand, thanks to the explicit syntax, we can use both `FormatStringWithDisplay`
and `FormatStringWithDebug` on `Person` without any issue.

Nevertheless, having to explicitly pick a provider can be problematic, especially
if there are multiple providers to choose from. In the next chapter, we will look
at how we can link a provider trait with a consumer trait, so that we can use back
the simple `person.format_string()` syntax without needing to know which provider
to choose from.

## Beyond String Formatting

In this chapter, we make use of a very simplified example of formatting strings to
demonstrate the use case of provider traits. Our example may seem a bit redundant,
as it does not simplify the code much as compared to directly using `format!()`
to format the string with either `Debug` or `Display`.

However, similar pattern can be more useful in more complex use cases, such as
implementing [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html),
or even the `Display` trait itself. If we were to implement these traits using CGP,
we would also define provider traits such as follows:

```rust
# extern crate serde;
#
use core::fmt;
use serde::Serializer;

pub trait ProvideSerialize<Context> {
    fn serialize<S: Serializer>(context: &Context, serializer: S) -> Result<S::Ok, S::Error>;
}

pub trait ProvideFormat<Context> {
    fn fmt(context: &Context, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>;
}
```

As we can see above, we can define provider traits for any existing traits by replacing
the `Self` type with an explicit `Context` type. In this chapter, we would not be covering
the details on how to use CGP and provider traits to simplify formatting and serialization
implementations, as that is beyond the current scope. Suffice to say, as we go through
later chapters, it will become clearer on how having provider traits can impact us on
thinking about how to structure and implement modular code.