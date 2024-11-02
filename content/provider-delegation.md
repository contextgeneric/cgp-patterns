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

## Implementing Provider for Multiple Traits

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
string. Compared to before, we also make the methods return a `Result` to
handle errors during formatting and parsing. [^error] [^encoding]

Next, we also define the provider traits as follows:

```rust
# extern crate anyhow;
#
# use anyhow::Error;
#
pub trait StringFormatter<Context> {
    fn format_to_string(context: &Context) -> Result<String, Error>;
}

pub trait StringParser<Context> {
    fn parse_from_string(raw: &str) -> Result<Context, Error>;
}
```

Using the provider traits, we can implement context-generic providers
that


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
