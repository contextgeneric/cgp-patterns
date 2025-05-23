# Introduction

This book covers the design patterns for _context-generic programming_ (CGP),
a new programming paradigm for Rust that allows strongly-typed components
to be implemented and composed in a modular, generic, and type-safe way.

## What is Context-Generic Programming

A high level overview of CGP is available on the [project website](https://www.contextgeneric.dev/).
This section contains a summarized version of the overview.

At its core, CGP makes use of Rust's trait system to build generic
component _interfaces_ that decouple code that _consumes_ an interface
from code that _implements_ an interface.
Through this decoupling, code can be written to be generic over any context,
and then be wired to be used on a concrete context by writing few lines of code.
CGP makes use of Rust's strong type system to help ensure that any such
wiring is _type-safe_, catching any unsatisfied dependencies as compile-time errors.

CGP shares some similarities with other modular programming patterns, such as
OCaml modules, Scala implicits, mixins, and dependency injection. Compared to
these other patterns, CGP has a unique advantage that it enables high modularity
while also being type-safe and concise. With Rust as its host language, CGP
also allows high-performance and low-level code to be written in a modular
way, without requiring complex runtime support.

CGP is designed to solve a wide range of common problems in programming,
including error handling, logging, encoding, and modular dispatch.
In particular, it allows writing static-typed Rust programs to be almost
as expressive as writing dynamic-typed programs, but with the additional
type safety guarantees.
If you ever feel that Rust's type system is restricting your ability to reuse
code, or be forced to duplicate code through copy-pasting or macros, then
CGP may be of help to solve your problem.

That said, programming in CGP is as expressive, but not as easy,
as dynamic-typed programming. There may be a steep learning curve
in learning how to program in a generic way, and this book aims to help
make that learning process more approachable.
Thoughout this book, we will slowly understand how CGP works, and learn about
useful design patterns that can be used in any programming situation.

## Work In Progress

This book is currently a work in progress. A majority of the chapter is yet to be written.
Please come back later to check out a completed version of this book.

## Who This Book Is For

This book takes a deep dive into how CGP works, building and explaining its core constructs from the ground up.

However, it's worth noting that many of the concepts explained here are not immediately essential for simply writing working CGP code, particularly when you are just starting out. This is because CGP provides a high level of abstraction, allowing developers to write highly productive code without needing to understand all the underlying details.

<div class="warning">
If your primary goal is to start writing CGP code and become productive as quickly as possible, this book might not be the most direct path.
</div>

On the other hand, this book is primarily intended for those who wish to gain a better understanding of what happens "behind the macros" after they have gained some experience using CGP.

It will also be a valuable resource for users encountering CGP-related error messages, providing the knowledge needed to understand the root cause and fix issues more effectively.

For beginners looking for a faster start, a separate book is planned for the future, which will provide more beginner-friendly tutorials focused on quickly getting started with CGP.
book will be written in the future, to provide beginner-friendly tutorials for getting started with CGP more quickly.

## Chapter Outlines

The first section of this book, _Terminology_, will introduce common terms to be used to understand CGP.
We will learn about what is a context, and what are consumer and provider traits.

In the next section, _Core Concepts_, we will cover the core concepts and the Rust-specific
design patterns that we use to enable context-generic programming.

Following that, _Design Patterns_ will introduce general design patterns that are built on top of the
foundation of context-generic programming.

The section _Domain-Specific Patterns_ will cover use-case-specific design patterns, such as error handling and logging.

Finally, the secion _Related Concepts_ will compare context-generic programming with related concepts,
such as the similarity and differences of context-generic programming as compared to object-oriented programming.

## Contribution

This book is open sourced under the MIT license on [GitHub](https://github.com/contextgeneric/cgp-patterns).

Anyone is welcome to contribute by submitting [pull requests](https://github.com/contextgeneric/cgp-patterns/pulls)
for grammatical correction, content improvement, or adding new design patterns.

A [GitHub Discussions](https://github.com/contextgeneric/cgp-patterns/discussions) forum is available for readers
to ask questions or have discussions for topics covered in this book.
