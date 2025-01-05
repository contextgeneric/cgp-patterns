# Error Wrapping

When programming in Rust, there is a common need to not only raise new errors, but also
attach additional details to an error that has previously been raised.
This is mainly to allow a caller to attach additional details about which higher-level
operations are being performed, so that better error report and diagnostics can be
presented to the user.

As an example, supposed that an application encountered a network error when communicating
to an external service, it may want to attach additional details that the error
occured when it tries to perform a particular action, such as to authenticate a user.

Error libraries such as `anyhow` and `eyre` provide methods such as
[`context`](https://docs.rs/anyhow/latest/anyhow/struct.Error.html#method.context) and
[`wrap_err`](https://docs.rs/eyre/latest/eyre/struct.Report.html#method.wrap_err)
to allow wrapping of additional details to their error type.
In this chapter, we will discuss about how to implement context-generic error wrapping
with CGP, and how to integrate them with existing error libraries.



## Wrapped Source Error

With the same motivation described in the [previous chapter](./error-reporting.md),
we would like to make use of CGP to also enable modular error reporting for the
error details that is being wrapped. This would mean that we want to define a
generic `Detail` type that can include _structured data_ inside the error
details. When combined with the abstract error type, we would first define
a wrapper type `WrapError` to wrap the detail with the error:

```rust
pub struct WrapError<Detail, Error> {
    pub detail: Detail,
    pub error: Error,
}
```

We can then use `WrapError` inside `CanRaiseError`, to wrap additional error details
in the form of `CanRaiseError<WrapError<Detail, Context::Error>>`.