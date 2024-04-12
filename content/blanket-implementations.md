# Blanket Trait Implementations

In the previous chapter, we have an implementation of `CanGreet` for `Person` that
makes use of `HasName` to retrieve the person's name to be printed.
However, the implementation is _context-specific_ to the `Person` context,
and cannot be reused for other contexts.

Ideally, we want to be able to define _context-generic_ implementations
of `Greet` that works with any context type that also implements `HasName`.
For this, the _blanket trait implementations_ pattern is one basic way which we can use for
defining context-generic implementations:

```rust
trait HasName {
    fn name(&self) -> &str;
}

trait CanGreet {
    fn greet(&self);
}

impl<Context> CanGreet for Context
where
    Context: HasName,
{
    fn greet(&self) {
        println!("Hello, {}!", self.name());
    }
}
```

The above example shows a blanket trait implementation of `CanGreet` for any
`Context` type that implements `HasName`. With that, contexts like `Person`
do not need to explicitly implement `CanGreet`, if they already implement
`HasName`:

```rust
# trait HasName {
#     fn name(&self) -> &str;
# }
#
# trait CanGreet {
#     fn greet(&self);
# }
#
# impl<Context> CanGreet for Context
# where
#     Context: HasName,
# {
#     fn greet(&self) {
#         println!("Hello, {}!", self.name());
#     }
# }
#
struct Person { name: String }

impl HasName for Person {
    fn name(&self) -> &str {
        &self.name
    }
}

let person = Person { name: "Alice".to_owned() };
person.greet();
```

As shown above, we are able to call `person.greet()` without having a context-specific
implementation of `CanGreet` for `Person`.

The use of blanket trait implementation is commonly found in many Rust libraries today.
For example, [`Itertools`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html)
provides a blanket implementation for any context that implements `Iterator`.
Another example is [`StreamExt`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html),
which is implemented for any context that implements `Stream`.

## Overriding Blanket Implementations

Traits containing blanket implementation are usually not meant to be implemented manually
by individual contexts. They are usually meant to serve as convenient methods that extends the
functionality of another trait. However, Rust's trait system does _not_ completely prevent us
from overriding the blanket implementation.

Supposed that we have a `VipPerson` context that we want to implement a different way of
greeting the VIP person. We could override the implementation as follows:

```rust
trait HasName {
    fn name(&self) -> &str;
}

trait CanGreet {
    fn greet(&self);
}

impl<Context> CanGreet for Context
where
    Context: HasName,
{
    fn greet(&self) {
        println!("Hello, {}!", self.name());
    }
}

struct VipPerson { name: String, /* other fields */ }

impl CanGreet for VipPerson {
    fn greet(&self) {
        println!("A warm welcome to you, {}!", self.name);
    }
}
```

The example above shows _two_ providers of `CanGreet`. The first provider is
a context-generic provider that we covered previously, but the second provider
is a context-specific provider for the `VipPerson` context.

## Conflicting Implementations

In the previous example, we are able to define a custom provider for `VipPerson`,
but with an important caveat: that `VipPerson` does _not_ implement `HasName`.
If we try to define a custom provider for contexts that already implement `HasName`,
such as for `Person`, the compilation would fail:

```rust,compile_fail
trait HasName {
    fn name(&self) -> &str;
}

trait CanGreet {
    fn greet(&self);
}

impl<Context> CanGreet for Context
where
    Context: HasName,
{
    fn greet(&self) {
        println!("Hello, {}!", self.name());
    }
}

struct Person { name: String }

impl HasName for Person {
    fn name(&self) -> &str {
        &self.name
    }
}

impl CanGreet for Person {
    fn greet(&self) {
        println!("Hi, {}!", self.name());
    }
}
```

If we try to compile the example code above, we would get an error with the message:

```text
conflicting implementations of trait `CanGreet` for type `Person`
```

The reason for the conflict is because Rust trait system requires all types
to have unambigious implementation of any given trait. To see why such requirement
is necessary, consider the following example:

```rust
# trait HasName {
#     fn name(&self) -> &str;
# }
#
# trait CanGreet {
#     fn greet(&self);
# }
#
# impl<Context> CanGreet for Context
# where
#     Context: HasName,
# {
#     fn greet(&self) {
#         println!("Hello, {}!", self.name());
#     }
# }
#
fn call_greet_generically<Context>(context: &Context)
where
    Context: HasName,
{
    context.greet()
}
```

The example above shows a generic function `call_greet_generically`, which work with
any `Context` that implements `HasName`. Even though it does not require `Context` to
implement `CanGreet`, it nevertheless can call `context.greet()`. This is because with
the guarantee from Rust's trait system, the compiler can always safely use the blanket
implementation of `CanGreet` during compilation.

If Rust were to allow ambiguous override of blanket implementations, such as what we
tried with `Person`, it would have resulted in inconsistencies in the compiled code,
depending on whether it is known that the generic type is instantiated to `Person`.

Note that in general, it is not always possible to know locally about the concrete type
that is instantiated in a generic code. This is because a generic function like
`call_greet_generically` can once again be called by other generic code. This is why
even though there are unstable Rust features such as
[_trait specialization_](https://rust-lang.github.io/rfcs/1210-impl-specialization.html),
such feature has to be carefully considered to ensure that no inconsistency can arise.

## Limitations of Blanket Implementations

Due to potential conflicting implementations, the use of blanket implementations offer
limited customizability, in case if a context wants to have a different implementation.
Although a context many define its own context-specific provider to override the blanket
provider, it would face other limitations such as not being able to implement other traits
that may cause a conflict.

In practice, we consider that blanket implementations allow for _singular context-generic provider_
to be defined. In future chapters, we will look at how to relax the singular constraint,
to make it possible to allow _multiple_ context-generic or context-specific providers to co-exist.