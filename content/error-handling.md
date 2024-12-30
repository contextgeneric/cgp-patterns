# Error Handling

Rust provides a relatively new way of handling errors, with the use of `Result` type
to represent explicit errors. Compared to the practice of implicit exceptions in other
mainstream languages, the explicit `Result` type provides many advantages, such as
making it clear when and what kind of errors can occur when calling a function.
However, until now there is not yet a clear consensus of which _error type_ should
be used within a `Result`.

The reason why choosing an error type is complicated is often due to different
applications having different concerns: Should the error capture stack traces?
Can the error be used in no_std environment? How should the error message be
displayed? Should the error contain _structured metadata_ that can be introspected
or logged differently? How should one differentiate different errors to decide
whether to retry an operation? How to compose or _flatten_ error sources that
come from using different libraries? etc.

Due to the complex cross-cutting concerns, there are never-ending discussions
across the Rust communities on the quest to find a perfect error type that
can be used to solve _all_ error handling problems. At the moment, the
Rust ecosystem leans toward using error libraries such as
[`anyhow`](https://docs.rs/anyhow) to store error values using some
form of _dynamic typing_. However, these approaches give up some of the
advantages provided by static types, such as the ability to statically
know whether a function would never raise certain errors.

CGP offers us an alternative approach towards error handling, which is
to use _abstract_ error types in `Result`, together with a context-generic
way of _raising errors_ without access to the concrete type.
In this chapter, we will walk through this new approach of error handling,
and look at how it allows error handling to be easily customized depending
on the exact needs of an application.