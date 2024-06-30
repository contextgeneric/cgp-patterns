# Impl-side Dependencies

When writing generic code, we often need to specify the trait bounds that
we would like to use with a generic type. However, when the trait bounds
involve traits that make use of blanket implementations, there are different
ways that we can specify the trait bounds.

Supposed that we want to define a generic function that formats a list of items
into a comma-separated string. Our generic function could make use the
[`Itertools::join`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.join)
to format an iterator. Our first attempt would be to define our generic function
as follows:


```rust
# extern crate core;
# extern crate itertools;

use core::fmt::Display;
use itertools::Itertools;

fn format_iter<I>(mut iter: I) -> String
where
    I: Iterator,
    I::Item: Display,
{
    iter.join(", ")
}

assert_eq!(format_iter(vec![1, 2, 3].into_iter()), "1, 2, 3");
```

The generic function `format_iter` takes a generic type `I` that implements
`Iterator`. Additionally, we require `I::Item` to implement `Display`. With both
constraints in place, we are able to call `Itertools::join` inside the generic
function to join the items using `", "` as separator.

In the above example, we are able to use the method from `Itertools` on `I`,
even though we do not specify the constraint `I: Itertools` in our `where` clause.
This is made possible because the trait `Itertools` has a blanket implementation
on all types that implement `Iterator`, including the case when we do not know
the concrete type behind `I`. Additionally, the method `Itertools::join` requires
`I::Item` to implement `Display`, so we also include the constraint in our where clause.

When using traits that have blanket implementation, we can also go the other way
and require `I` to implement `Itertools` instead of `Iterator`:


```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;

fn format_iter<I>(mut items: I) -> String
where
    I: Itertools,
    I::Item: Display,
{
    items.join(", ")
}

assert_eq!(format_iter(vec![1, 2, 3].into_iter()), "1, 2, 3");
```

By doing so, we make it explicit of the intention that we only care that
`I` implements `Itertools`, and hide the fact that we need `I` to also implement
`Iterator` in order to implement `Itertools`.

## Constraint Leaks

At this point, we have defined our generic function `format_iter` with two constraints
in the `where` clause. When calling `format_iter` from another generic function, the
constraint would also be propagated to the caller.

As a demonstration, supposed that we want to define another generic function that
uses `format_iter` to format any type that implements `IntoIterator`, we would need
to also include the constraints needed by `format_iter` as follows:


```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# fn format_iter<I>(mut items: I) -> String
# where
#     I: Itertools,
#     I::Item: Display,
# {
#     items.join(", ")
# }

fn format_items<C>(items: C) -> String
where
    C: IntoIterator,
    C::IntoIter: Itertools,
    <C::IntoIter as Iterator>::Item: Display,
{
    format_iter(items.into_iter())
}

assert_eq!(format_items(&vec![1, 2, 3]), "1, 2, 3");
```

When defining the generic function `format_items` above, we only really care
that the generic type `C` implements `IntoIterator`, and then pass `C::IntoIter`
to `format_iter`. However, because of the constraints specified by `format_iter`,
Rust also forces us to specify the same constraints in `format_items`, even if
we don't need the constraints directly.

As we can see, the constraints specified in the `where` clause of `format_iter`
is a form of leaky abstraction, as it also forces generic consumers like
`format_items` to also know about the internal details of how `format_iter`
uses the iterator.

The leaking of `where` constraints also makes it challenging to write highly
generic functions at a larger scale. The number of constraints could quickly
become unmanageable, if a high level generic function calls many low-level
generic functions that each has different constraints.

Furthermore, the repeatedly specified constraints become tightly coupled with
the concrete implementation of the low-level functions. For example, if
`format_iter` changed from using `Itertools::join` to other ways of formatting
the iterator, the constraints would become outdated and need to be changed
in `format_items`.


## Hiding Constraints with Traits and Blanket Implementations

Using the techniques we learned from blanket implementations, there is a way to
hide the `where` constraints by redefining our generic functions as traits with
blanket implementations.

We would first rewrite `format_iter` into a trait `CanFormatIter` as follows:

```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;

pub trait CanFormatIter {
    fn format_iter(self) -> String;
}

impl<I> CanFormatIter for I
where
    I: Itertools,
    I::Item: Display,
{
    fn format_iter(mut self) -> String
    {
        self.join(", ")
    }
}

assert_eq!(vec![1, 2, 3].into_iter().format_iter(), "1, 2, 3");
```

The trait `CanFormatIter` is defined with a single method, `format_iter`, which
consumes `self` and return a `String`. The trait comes with a blanket implementation
for any type `I`, with the constraints that `I: Itertools` and `I::Item: Display`.
Following that, we have the same implementation as before, which calls `Itertools::join`
to format the iterator as a comma-separated string. By having a blanket implementation,
we signal that `CanFormatIter` is intended to be derived automatically, and that no
explicit implementation is required.

It is worth noting that the constraints `I: Itertools` and `I::Item: Display` are only
present at the `impl` block, but not at the `trait` definition of `CanFormatIter`.
By doing so, we have effectively "hidden" the constraints inside the `impl` block,
and prevent it from leaking to its consumers.

We can now refactor `format_items` to use `CanFormatIter` as follows:

```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# pub trait CanFormatIter {
#     fn format_iter(self) -> String;
# }
#
# impl<I> CanFormatIter for I
# where
#     I: Itertools,
#     I::Item: Display,
# {
#     fn format_iter(mut self) -> String
#     {
#         self.join(", ")
#     }
# }

fn format_items<C>(items: C) -> String
where
    C: IntoIterator,
    C::IntoIter: CanFormatIter,
{
    items.into_iter().format_iter()
}

assert_eq!(format_items(&vec![1, 2, 3]), "1, 2, 3");
```

In the new version of `format_items`, our `where` constraints are now simplified
to only require `C::IntoIter` to implement `CanFormatIter`. With that, we are
able to make it explicit that `format_items` needs `CanFormatIter` to be implemented,
but it doesn't matter _how_ it is implemented.

The reason this technique works is similar to how we used `Itertools` in our
previous examples. At the call site of the code that calls `format_items`, Rust
would see that the generic function requires `C::IntoIter` to implement
`CanFormatIter`. But at the same time, Rust also sees that `CanFormatIter` has
a blanket implementation. So if the constraints specified at the blanket
implementation are satisfied, Rust would automatically provide an implementation
of `CanFormatIter` to `format_items`, without the caller needing to know how
that is done.


## Nested Constraints Hiding

Once we have seen in action how we can hide constraints behind the blanket `impl`
blocks of traits, there is no stopping for us to define more traits with blanket
implementations to hide even more constraints.

For instance, we could rewrite `format_items` into a `CanFormatItems` trait as follows:

```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# pub trait CanFormatIter {
#     fn format_iter(self) -> String;
# }
#
# impl<I> CanFormatIter for I
# where
#     I: Itertools,
#     I::Item: Display,
# {
#     fn format_iter(mut self) -> String
#     {
#         self.join(", ")
#     }
# }

pub trait CanFormatItems {
    fn format_items(&self) -> String;
}

impl<Context> CanFormatItems for Context
where
    for<'a> &'a Context: IntoIterator,
    for<'a> <&'a Context as IntoIterator>::IntoIter: CanFormatIter,
{
    fn format_items(&self) -> String
    {
        self.into_iter().format_iter()
    }
}

assert_eq!(vec![1, 2, 3].format_items(), "1, 2, 3");
```

We first define a `CanFormatItems` trait, with a method `format_items(&self)`.
Here, we make an improvement over the original function to allow a reference `&self`,
instead of an owned value `self`. This allows a container such as `Vec` to
not be consumed when we try to format its items, which would be unnecessarily
inefficient.

Inside the blanket `impl` block for `CanFormatItems`, we define it to work with any
`Context` type, given that the generic `Context` type implements some constraints
with [_higher ranked trait bounds_ (HRTB)](https://doc.rust-lang.org/nomicon/hrtb.html).
While HRTB is an advanced subject on its own, the general idea is that we require
that any reference `&'a Context` with any lifetime `'a` implements `IntoIterator`.
This is so that when
[`IntoIterator::into_iter`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html#tymethod.into_iter)
is called, the `Self` type being consumed is the reference type `&'a Context`,
which is implicitly copyable, and thus allow the same context to be reused later
on at other places.

Additionally, we require that `<&'a Context as IntoIterator>::IntoIter` implements
`CanFormatIter`, so that we can call its method on the produced iterator. Thanks to
the hiding of constraints by `CanFormatIter`, we can avoid specifying an overly verbose
constraint that the iterator item also needs to implement `Display`.

Individually, the constraints hidden by `CanFormatIter` and `CanFormatItems` may
not look significant. But when combining together, we can see how isolating the
constraints help us better organize our code and make them cleaner.
In particular, we can now write generic functions that consume `CanFormatIter`
without having to understand all the indirect constraints underneath.

To demonstrate, supposed that we want to compare two list of items and see
whether they have the same string representation. We can now define a
generic `stringly_equals` function as follows:

```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# pub trait CanFormatIter {
#     fn format_iter(self) -> String;
# }
#
# impl<I> CanFormatIter for I
# where
#     I: Itertools,
#     I::Item: Display,
# {
#     fn format_iter(mut self) -> String
#     {
#         self.join(", ")
#     }
# }
#
# pub trait CanFormatItems {
#     fn format_items(&self) -> String;
# }
#
# impl<Context> CanFormatItems for Context
# where
#     for<'a> &'a Context: IntoIterator,
#     for<'a> <&'a Context as IntoIterator>::IntoIter: CanFormatIter,
# {
#     fn format_items(&self) -> String
#     {
#         self.into_iter().format_iter()
#     }
# }

fn stringly_equals<Context>(left: &Context, right: &Context) -> bool
where
    Context: CanFormatItems,
{
    left.format_items() == right.format_items()
}

assert_eq!(stringly_equals(&vec![1, 2, 3], &vec![1, 2, 4]), false);
```

Our generic function `stringly_equals` can now be defined cleanly to work over
any `Context` type that implements `CanFormatItems`. In this case, the function
does not even need to be aware that `Context` needs to produce an iterator, with
its items implementing `Display`.

Furthermore, instead of defining a generic function, we could instead use the
same programming technique and define a `CanStringlyCompareItems` trait
that does the same thing:

```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# pub trait CanFormatIter {
#     fn format_iter(self) -> String;
# }
#
# impl<I> CanFormatIter for I
# where
#     I: Itertools,
#     I::Item: Display,
# {
#     fn format_iter(mut self) -> String
#     {
#         self.join(", ")
#     }
# }
#
# pub trait CanFormatItems {
#     fn format_items(&self) -> String;
# }
#
# impl<Context> CanFormatItems for Context
# where
#     for<'a> &'a Context: IntoIterator,
#     for<'a> <&'a Context as IntoIterator>::IntoIter: CanFormatIter,
# {
#     fn format_items(&self) -> String
#     {
#         self.into_iter().format_iter()
#     }
# }

pub trait CanStringlyCompareItems {
    fn stringly_equals(&self, other: &Self) -> bool;
}

impl<Context> CanStringlyCompareItems for Context
where
    Context: CanFormatItems,
{
    fn stringly_equals(&self, other: &Self) -> bool {
        self.format_items() == other.format_items()
    }
}

assert_eq!(vec![1, 2, 3].stringly_equals(&vec![1, 2, 4]), false);
```

For each new trait we layer on top, we can build higher level interfaces that
hide away lower level implementation details. When `CanStringlyCompareItems` is
used, the consumer is shielded away from knowing anything about the concrete
context, other than that two values are be compared by first being formatted
into strings.



The example here may seem a bit stupid, but there are some practical use cases of
implementing comparing two values as strings. For instance, a serialization library
may want to use it inside tests to check whether two different values are serialized
into the same string. For such use case, we may want to define another trait to
help make such assertion during tests:


```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# pub trait CanFormatIter {
#     fn format_iter(self) -> String;
# }
#
# impl<I> CanFormatIter for I
# where
#     I: Itertools,
#     I::Item: Display,
# {
#     fn format_iter(mut self) -> String
#     {
#         self.join(", ")
#     }
# }
#
# pub trait CanFormatItems {
#     fn format_items(&self) -> String;
# }
#
# impl<Context> CanFormatItems for Context
# where
#     for<'a> &'a Context: IntoIterator,
#     for<'a> <&'a Context as IntoIterator>::IntoIter: CanFormatIter,
# {
#     fn format_items(&self) -> String
#     {
#         self.into_iter().format_iter()
#     }
# }
#
# pub trait CanStringlyCompareItems {
#     fn stringly_equals(&self, other: &Self) -> bool;
# }
#
# impl<Context> CanStringlyCompareItems for Context
# where
#     Context: CanFormatItems,
# {
#     fn stringly_equals(&self, other: &Self) -> bool {
#         self.format_items() == other.format_items()
#     }
# }
#
pub trait CanAssertEqualImpliesStringlyEqual {
    fn assert_equal_implies_stringly_equal(&self, other: &Self);
}

impl<Context> CanAssertEqualImpliesStringlyEqual for Context
where
    Context: Eq + CanStringlyCompareItems,
{
    fn assert_equal_implies_stringly_equal(&self, other: &Self) {
        assert_eq!(self == other, self.stringly_equals(other))
    }
}

vec![1, 2, 3].assert_equal_implies_stringly_equal(&vec![1, 2, 3]);
vec![1, 2, 3].assert_equal_implies_stringly_equal(&vec![1, 2, 4]);
```

The trait `CanAssertEqualImpliesStringlyEqual` provides a method that
takes two contexts of the same type, and assert that if both context
values are equal, then their string representation are also equal.
Inside the blanket `impl` block, we require that `Context` implements
`CanStringlyCompareItems`, as well as `Eq`.

Thanks to the hiding of constraints, the trait `CanAssertEqualImpliesStringlyEqual`
can cleanly separate its direct dependencies, `Eq`, from the rest of the
indirect dependencies.

## Dependency Injection

The programming technique that we have introduced in this chapter is sometimes
known as _dependency injection_ in some other languages and programming paradigms.
The general idea is that the `impl` blocks are able to specify the dependencies
they need in the form of `where` constraints, and the Rust trait system automatically
helps us to resolve the dependencies at compile time.

In context-generic programming, we think of constraints in the `impl` blocks not as
constraints, but more generally _dependencies_ that the concrete implementation needs.
Each time a new trait is defined, it serves as an interface for consumers to include
them as a dependency, but at the same time separates the declaration from the concrete
implementations.