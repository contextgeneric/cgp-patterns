# Provider

In CGP, a _provider_ is a piece of code that _implements_ certain functionality
for a context. At its most basic, a provider is consist of an `impl` block for
a trait.

```rust
trait HasName {
    fn name(&self) -> &str;
}

struct Person { name: String }

impl HasName for Person {
    fn name(&self) -> &str {
        &self.name
    }
}
```

In the above example, we implement the `HasName` for the `Person` struct.
The block `impl HasName for Person` is a _provider_ of the `HasName` trait
for the `Person` context.

Similar to the concept of a consumer, the use of provider is common in any
Rust code that implements a trait. However, compared to cosumers, there
are limitations on how providers can be defined in Rust.

For this example, the `impl` block is a _context-specific_ provider for the
`Person` context. Furthermore, due to the restrictions of Rust's trait system,
there can be at most one provider of `HasName` for the `Person` context.
Another common restriction is that the provider has to be defined in the same
crate as either the trait or the context.

The asymetry between what can be done with a provider, as compared to a consumer,
is often a source of complexity in many Rust programs. As we will learn in later chapters,
one of the goals of CGP is to break this asymetry, and make it easy to implement
_context-generic providers_.

## Providers as Consumers

Although we have providers and consumers as distinct concepts, it is common to
have code that serve as _both_ providers and consumers.

```rust
# trait HasName {
#     fn name(&self) -> &str;
# }
#
# struct Person { name: String }
#
# impl HasName for Person {
#     fn name(&self) -> &str {
#         &self.name
#     }
# }
#
trait CanGreet {
    fn greet(&self);
}

impl CanGreet for Person {
    fn greet(&self) {
        println!("Hello, {}!", self.name());
    }
}
```

The example above shows a new `CanGreet` trait, which provides a `greet` method.
We then implement `CanGreet` for `Person`, with the `greet` implementation using
`self.name()` to print out the name to be greeted.

Here, the block `impl CanGreet for Person` is a provider of `CanGreet` for the `Person`
context. At the same time, it is also the _consumer_ of `HasName` for the `Person` context.
In terms of genericity, the example code is _context-specific_ to the `Person` context for both
the consumer and provider side.

As we will see in later chapters, a powerful idea introduced by CGP is that a piece of code
can have _multiple spectrums_ of genericity on the consumer and provider sides.