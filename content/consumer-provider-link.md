# Linking Consumers with Providers

In the [previous chapter](./provider-traits.md), we learned about how provider
traits allow multiple overlapping implementations to be defined. However, if
everything is implemented only as provider traits, it would be much more tedious
having to determine which provider to use, at every time when we need to use the
trait. To overcome this, we would need have _both_ provider traits and consumer
traits, and have some ways to choose a provider when implementing a consumer trait.

## Implementing Consumer Traits

The simplest way to link a consumer trait with a provider is by implementing the
consumer trait to call a chosen provider. Consider the `StringFormatter` example
of the previous chapter, we would implement `CanFormatString` for a `Person`
context as follows:

```rust
use core::fmt::{self, Display};

pub trait CanFormatString {
    fn format_string(&self) -> String;
}

pub trait StringFormatter<Context> {
    fn format_string(context: &Context) -> String;
}

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

impl CanFormatString for Person {
    fn format_string(&self) -> String {
        FormatStringWithDisplay::format_string(self)
    }
}

let person = Person { first_name: "John".into(), last_name: "Smith".into() };

assert_eq!(person.format_string(), "John Smith");
#
# pub struct FormatStringWithDisplay;
#
# impl<Context> StringFormatter<Context> for FormatStringWithDisplay
# where
#     Context: Display,
# {
#     fn format_string(context: &Context) -> String {
#         format!("{}", context)
#     }
# }
```

To recap the previous chapter, we have a consumer trait `CanFormatString`
and a provider trait `StringFormatter`. There are two example providers that
implemenent `StringFormatter` - `FormatStringWithDisplay` which formats strings
using `Display`, and `FormatStringWithDebug` which formats strings using `Debug`.
In addition to that, we implement `CanFormatString` for the `Person` context
by forwarding the call to `FormatStringWithDisplay`.

By doing so, we effectively "bind" the `StringFormatter` provider for the
`Person` context to `FormatStringWithDisplay`. With that, any time a consumer
code calls `person.format_string()`, it would automatically format the context
using `Display`.

Thanks to the decoupling of providers and consumers, a context like `Person`
can freely choose between multiple providers, and link them with relative ease.
Similarly, the provider trait allows multiple context-generic providers such as
`FormatStringWithDisplay` and `FormatStringWithDebug` to co-exist.

## Blanket Consumer Trait Implementation

In the previous section, we manually implemented `CanFormatString` for `Person`
with an explicit call to `FormatStringWithDisplay`. Although the implementation
is relatively short, it can become tedious if we make heavy use of provider traits,
which would require us to repeat the same pattern for every trait.

To simplify this further, we can make use of _blanket implementations_ to
automatically delegate the implementation of _all_ consumer traits to one
chosen provider. We would define the blanket implementation for `CanFormatString`
as follows:

```rust
pub trait HasProvider {
    type Components;
}

pub trait CanFormatString {
    fn format_string(&self) -> String;
}

pub trait StringFormatter<Context> {
    fn format_string(context: &Context) -> String;
}

impl<Context> CanFormatString for Context
where
    Context: HasProvider,
    Context::Components: StringFormatter<Context>,
{
    fn format_string(&self) -> String {
        Context::Components::format_string(self)
    }
}
```

First of all, we define a new `HasProvider` trait that contains an associated
type `Components`. The `Components` type would be specified by a context to
choose a provider that it would use to forward all implementations of consumer
traits. Following that, we add a blanket implementation for `CanFormatString`,
which would be implemented for any `Context` that implements `HasProvider`,
provided that `Context::Components` implements `StringFormatter<Context>`.

To explain in simpler terms - if a context has a provider that implements
a provider trait for that context, then the consumer trait for that context
is also automatically implemented.

With the new blanket implementation in place, we can now implement `HasProvider`
for the `Person` context, and it would now help us to implement `CanFormatString`
for free:

```rust
use core::fmt::{self, Display};

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

impl HasProvider for Person {
    type Provider = FormatStringWithDisplay;
}

let person = Person { first_name: "John".into(), last_name: "Smith".into() };

assert_eq!(person.format_string(), "John Smith");
#
# pub trait HasProvider {
#     type Components;
# }
#
# pub trait CanFormatString {
#     fn format_string(&self) -> String;
# }
#
# pub trait StringFormatter<Context> {
#     fn format_string(context: &Context) -> String;
# }
#
# impl<Context> CanFormatString for Context
# where
#     Context: HasProvider,
#     Context::Components: StringFormatter<Context>,
# {
#     fn format_string(&self) -> String {
#         Context::Components::format_string(self)
#     }
# }
#
# pub struct FormatStringWithDisplay;
#
# impl<Context> StringFormatter<Context> for FormatStringWithDisplay
# where
#     Context: Display,
# {
#     fn format_string(context: &Context) -> String {
#         format!("{}", context)
#     }
# }
```

Compared to before, the implementation of `HasProvider` is much shorter than
implementing `CanFormatString` directly, since we only need to specify the provider
type without any function definition.

At the moment, because the `Person` context only implements one consumer trait, we
can set `FormatStringWithDisplay` directly as `Person::Components`. However, if there
are other consumer traits that we would like to use with `Person`, we would need to
define `Person::Components` with a separate provider that implements multiple provider
traits. This will be covered in the next chapter, which we would talk about how to
link multiple providers of different provider traits together.

## Component System

You may have noticed that the trait for specifying the provider for a context is called
`HasProvider` instead of `HasProviders`. This is to generalize the idea of a pair of
consumer trait and provider trait working together, forming a _component_.

In context-generic programming, we use the term _component_ to refer to a consumer-provider
trait pair. The consumer trait and the provider trait are linked together through blanket
implementations and traits such as `HasProvider`. These constructs working together to
form the basis for a _component system_ for CGP.