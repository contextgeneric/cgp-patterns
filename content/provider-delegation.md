# Provider Delegation

In the previous chapter, we learned to make use of the `HasComponent` trait
to define a blanket implementation for a consumer trait like `CanFormatString`,
so that a context would automatically delegate the implementation to a provider
trait like `StringFormatter`. However, because there can only be one `Component`
type defined for `HasComponent`, this means that the given provider needs to
implement _all_ provider traits that we would like to use for the context.

In this chapter, we will learn to combine multiple providers that each implements
a distinct provider trait, and turn them into a single provider that implements
multiple provider traits.

## Implementing Provider for Multiple Traits

Consider that instead of just formatting a context as string,