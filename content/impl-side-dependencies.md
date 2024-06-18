# Impl-side Dependencies

When writing generic code, we often need to specify the trait bounds that
we would like to use with a generic type. However, when the trait bounds
involve traits that make use of blanket implementations, there are different
ways that we can specify the trait bounds.

As an example, supposed that we want to define a generic `format_items`
function that format a list of items into string. Our generic function could
make use the method
[`Itertools::join`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.join)
to join the iterator. With that, we may want to define our generic function as follows:


```rust
# extern crate core;
# extern crate itertools;

use core::fmt::Display;
use itertools::Itertools;

fn format_iter<I>(mut items: I) -> String
where
    I: Iterator,
    I::Item: Display,
{
    items.join(", ")
}

assert_eq!(format_iter(vec![1, 2, 3].into_iter()), "1, 2, 3");
```



```rust
# extern crate core;
# extern crate itertools;

use core::fmt::Display;
use itertools::Itertools;

fn format_iter<I>(mut items: I) -> String
where
    I: Itertools,
    I::Item: Display,
{
    items.join(", ")
}

assert_eq!(format_iter(vec![1, 2, 3].into_iter()), "1, 2, 3");
```


```rust
# extern crate core;
# extern crate itertools;

use core::fmt::Display;
use itertools::Itertools;

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
    C::Item: Display,
{
    items.into_iter().format_iter()
}

assert_eq!(format_items(&vec![1, 2, 3]), "1, 2, 3");
```



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

fn format_items<C>(items: &C) -> String
where
    for <'a> &'a C: IntoIterator,
    for <'a> <&'a C as IntoIterator>::IntoIter: CanFormatIter,
{
    items.into_iter().format_iter()
}

assert_eq!(format_items(&vec![1, 2, 3]), "1, 2, 3");
```



The `format_items` above works generically over any type `C` that implements
`IntoIterator`. Additionally, to use `Itertools::join`, we also require `C::Item`
to implement `Display`. With the trait bounds in place, we can simply call
`items.into_iter().join(", ")` to format the items as a comma-separated string.




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
