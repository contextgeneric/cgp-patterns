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

fn format_items<C>(items: C) -> String
where
    C: IntoIterator,
    C::Item: Display,
{
    items.into_iter().join(", ")
}

# assert_eq!(format_items(&vec![1, 2, 3]), "1, 2, 3");
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
# fn format_items<C>(items: C) -> String
# where
#     C: IntoIterator,
#     C::Item: Display,
# {
#     items.into_iter().join(", ")
# }

fn stringly_equals<C>(items_a: C, items_b: C) -> bool
where
    C: IntoIterator,
    C::Item: Display,
{
    format_items(items_a) == format_items(items_b)
}
```


```rust
# extern crate core;
# extern crate itertools;

use core::fmt::Display;
use itertools::Itertools;

pub trait CanFormatItems {
    fn format_items(&self) -> String;
}

impl<Context> CanFormatItems for Context
where
    Context: for<'a> IntoIterator<Item:Display>,
{
    fn format_items(&self) -> String
    {
        items.into_iter().join(", ")
    }
}
```



```rust
# extern crate core;
# extern crate itertools;
#
# use core::fmt::Display;
# use itertools::Itertools;
#
# pub trait CanFormatItems {
#     fn format_items(&self) -> String;
# }
#
# impl<Context> CanFormatItems for Context
# where
#     Context: for<'a> IntoIterator<Item:Display>,
# {
#     fn format_items(&self) -> String
#     {
#         items.into_iter().join(", ")
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
```

