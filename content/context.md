# Context

In CGP, we use the term _context_ to refer to a _type_ that provide certain functionalities, or dependencies.
The most common kind of functionality a context may provide is a _method_.

Following is a simple hello world example of a context providing a method:

```rust
struct MyContext;

impl MyContext {
    fn hello(&self) {
        println!("Hello World!");
    }
}
```

The example above is mostly self explanatory. We first define a struct called `MyContext`, followed
by an `impl` block for `MyContext`. Inside the `impl` block, a `hello` method is provided,
which prints out `"Hello World!"` to the terminal when called.

We can then use the `hello` method anywhere that we have a value of type `MyContext`, such as:

```rust
# struct MyContext;
#
# impl MyContext {
#     fn hello(&self) {
#         println!("Hello World!");
#     }
# }
#
let my_context = MyContext;

my_context.hello();
```

## Contexts vs Classes

The above example may seem trivial for most programmers, especially for those who come from object-oriented programming (OOP) background.
In fact, one way we can have a simplified view of a context is that it is similar to OOP concepts such as _classes_, _objects_, and _interfaces_.

Beyond the surface-level similarity, the concept of contexts in CGP is more general than classes and other similar concepts. As a result,
it is useful to think of the term _context_ as a new concept that we will learn in this book.

As we will learn in later chapters, aside from methods, a context may provide other kinds of functionalities, such as _associated types_
and _constants_.

## Contexts vs Types

Although a context is usually made of a type, in CGP we do not treat all types as contexts. Instead, we expect CGP contexts to offer
some level of _modularity_, which can be achived by using the programming patterns introduced in this book.