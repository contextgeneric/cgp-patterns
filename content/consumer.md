# Consumer

In CGP, a _consumer_ is a piece of code that consumes certain functionalities from a context.
There are several ways which a consumer may consume a functionality. At its most basic,
if a consumer has access to the _concrete_ type of a context, it can access any methods
defined by an `impl` block of that context.

```rust
struct Person { name: String }

impl Person {
    fn name(&self) -> &str {
        &self.name
    }
}

fn greet(person: &Person) {
    println!("Hello, {}!", person.name());
}
```

in the above example, we have a `greet` function that prints a greeting to a person using
the method `Person::name`. In other words, we say that the `greet` function is a _consumer_
to the `Person::name` method.

## Context-Generic Consumers

The `greet` function in our previous example can only work with the `Person` struct. However,
if we inspect the implementation of `greet`, we can see that it is possible to generalize
`greet` to work with _any_ type that has a name.

To generalize `greet`, we first need to define a _trait_ that acts as an _interface_ for getting
a name:

```rust
trait HasName {
    fn name(&self) -> &str;
}

fn greet<Context>(context: &Context)
where
    Context: HasName
{
    println!("Hello, {}", context.name());
}
```

In the example above, we define a `HasName` trait that provides a `name` method. We then redefine
`greet` to work generically with any `Context` type, with the `where` clause requiring `Context` to
implement `HasName`. Inside the function body, we call the `name` method, and print out the greeting
of that name.

Notice that in this example, we are able to implement `greet` _before_ we have any concrete implementation
of `HasName`. Compared to before, `greet` is now _decoupled_ from the `Person` type, thus making
our code more modular.

In CGP, this new version of `greet` is considered a _context-generic_ consumer, as it is able to _generically_
consume the `HasName::name` method from any `Context` type that implements `HasName`.

The concept of context-generic consumer is not unique to CGP. In fact, it is already commonly used
in most of the Rust code that uses traits. However, we make an effort to study this concept, so that
we can further generalize the concept in the later chapters of this book.