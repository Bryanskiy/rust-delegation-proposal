
- Feature Name: (fill me in with a unique ident, `delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Provide a syntactic sugar to automatically forward function calls.

TODO: maybe something else? Should be filled last anyway.

## Motivation
[motivation]: #motivation

TODO

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TODO

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This proposal introduces a new [item kind](https://doc.rust-lang.org/reference/items.html), the _delegation item_:

```diff
Item →
    OuterAttribute* ( VisItem | MacroItem )

    VisItem →
    Visibility
    (
        Module
      | ExternCrate
      ...
+     | Delegation
```

Delegation items can be declared in any position where items are allowed. They are also associated items and may therefore appear in traits and implementations ([?](#why-can-delegation-items-be-declared-in-any-position-and-why-are-qualified-paths-used-for-call-disambiguation)). Like other items, delegation items may be annotated with a visibility modifier ([?](#how-should-the-visibility-of-the-generated-function-be-determined)) and may have attributes applied to them.

The delegation item consists of a [fully qualified path](https://doc.rust-lang.org/reference/paths.html#qualified-paths) followed by a [block expression](https://doc.rust-lang.org/reference/expressions/block-expr.html):
```diff
+ Delegation →
+     QualifiedPathInExpression BlockExpression
```

Qualified paths provide an unambiguous way to identify callable items, including trait methods, trait implementation methods, inherent methods, and free functions.

TODO: why {}; `self` inside expression; generics; implementation notes about inherent methods; "refined"

TODO

## Drawbacks
[drawbacks]: #drawbacks

TODO

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

TODO

### Why can delegation items be declared in any position and why are qualified paths used for call disambiguation?

Delegation is fundamentally the forwarding of function calls. A regular function in Rust may be a trait method, a method in a trait implementation, an inherent method, or a free function. We can form different combinations based on the position of a caller and a callee:

- Delegating from a trait implementation to an implementation of the same trait.
- Delegating from a trait implementation to an implementation of another trait.
- Delegating from an inherent method to a trait implementation.
- Delegating from a free function to another free function.
- etc.

All these combinations appear in real world code via regular calls and each represents a potential target for the delegation feature. Choosing which combinations to support is a design decision driven by multiple factors: the function call resolution algorithm, the available syntax budget, the frequency of the use case, and the extensibility to other cases.

Rust distinguishes between two kinds of function invocation. The first one is [method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html), which resolves to associated methods that take a receiver argument. If more than one method is applicable the compiler emits an error. Se second kind is [fully qualified calls](https://doc.rust-lang.org/reference/expressions/call-expr.html#r-expr.call.desugar) which can be used to resolve the ambiguity.

From the delegation perspective we have a quite similar situation. [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) suggested to use method name only to resolve the callee. This covers the most common scenario: delegating a trait implementation to another implementation of the same trait. However this syntax does not generalize naturally to other caller/callee combinations since it can lead to ambiguities as already mentioned above.

Based on the provided information we can categorize the alternatives as follows:

1. TODO: The same as in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). But we would not propose the same thing twice and want to explore another rout.
2. TODO: Another approach, explored by [RFC 2018], is to use contextual keywords such as `trait`, `impl`, or `fn` as disambiguators.
3. TODO: We can leverage Rust's existing Fully Qualified Path (`<Type as Trait>::method`) syntax to refer to the callee. This approach covers every possible caller/callee combination without ambiguity, but it requires more verbose and explicit syntax.
4. TODO: We can use `target_expression` for resolution, i.e. typeof(`target_expression`) + name, but too complex from the implementation perspective. Add Note.

### How should the visibility of the generated function be determined?

 The proposed syntax allows the visibility of a reused function to be specified independently from the visibility of the original definition. While this flexibility is desirable, it introduces a potential _semver hazard_:

```rust
fn foo<T: Copy>(x: T) { /* impl */ }
pub reuse foo as bar;
```

If the signature of `foo` changes, the generated function `bar` changes accordingly. In regular Rust code, such a signature change would cause a type error at every call site, forcing the author to update callers. With `reuse`, however, the change propagates silently. As a result, modifications intended to be internal may accidentally become breaking changes for downstream crates.

Taking this into consideration, several design choices are possible:

1. The visibility of the generated function is taken solely from the callee.

2.  The visibility of the generated function cannot exceed the visibility of the reused function. In other words, delegation may only preserve or reduce visibility, never increase it. This can be implemented as a lint or a hard error.
3. Explicit visibility control by the user.

TODO: the choice

### Alternatives to this RFC

TODO

#### Reflection

An alternative approach to delegation in Rust would be some form of compile-time reflection. Given the ability to inspect type information such as function signatures during macro expansion, delegation can be implemented entirely as a third-party library, removing the need for dedicated language support.

Work in this direction is already being explored. See [reflection project goal](https://github.com/rust-lang/rust-project-goals/issues/406).

TODO: this is more general, but unclear whether can be implemented in Rust at all?

## Prior art
[prior-art]: #prior-art

TODO: add macro based crates

TODO: the whole point of previous RFCs analysis is to point out that there were issues with forward compatibility and in new proposal we solve this by better "design space exploration".

### [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) (2015)

Delegation was first proposed in a [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406). This RFC introduces a new syntax within trait `impl` blocks, permitting a type to forward an entire trait implementation (or selected items) to a field or arbitrary expression that already implements that trait. The proposed syntax takes the forms:
- `impl Trait for Type { use expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { use expression for name_1 (, name_i)*; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a value implementing `Trait`.

Prohibited patterns: delegation of associated constants ([?]()), delegation of associated types ([?]()).<br>
Proposed extensions: renaming ([?]()), `Self` type mapping ([?]()), multiple traits ([?]()),  delegation of enums ([?]()), arbitrary parent context ([?]()).

#### Main reasons for proposal rejection

_Unclear semantics._ It's not clear what kinds of expressions are allowed in the delegation body. Underspecified `self` behavior: callee might have no receiver, might take receiver by value(`self: Self`), by reference (`self: &Self`), by mut reference(`self: &mut Self`) or even more complex types after introduction of `arbitrary_self_types` feature. The mechanism for callee resolution is not defined.

TODO: mechanism for desugaring is not defined

_Forward compatibility._ The RFC intentionally leaves many features for future work, but there was insufficient evidence that the proposed design could be clearly extended to those features without breaking semantics and requiring a redesign.

You can also check Boat's [summary](https://github.com/rust-lang/rfcs/pull/1406#issuecomment-269175112).

TODO: we can add links to comments for each concern. Should we?

### [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) (2018)

Delegation was proposed again in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). The design was stricter to address the semantics ambiguities of the earlier proposal. The syntax takes the forms:
- `impl Trait for Type { delegate * to expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { delegate fn name_1 (, fn name_i)* to expression; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a field of `self` (e.g., `self.field`) and `typeof(expression)` implements `Trait`.

Delegation is allowed only for methods that take a receiver by value, by reference or by mutable reference. Other cases are left for future extensions. Proposed desugaring scheme translates delegation item into method call([?](https://doc.rust-lang.org/reference/expressions/method-call-expr.html)) and therefore follows the corresponding method resolution scheme.

Proposed extensions:
- delegation of associated constants ([?]()), delegation of associated types ([?]()).
> [!NOTE]
> Types and functions/consts exist in separate namespaces. Therefore `fn`, `const` and `type` keywords were proposed to be used before callee names in order to disambiguate them.
- TODO
- misc (e.g. arbitrary expressions)

#### Main reasons for proposal rejection

The second proposal was [postponed](https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011) due to the lang team bandwidth.

TODO: Mention that forward compatibility concern wasn't addressed. Should we provide example or it's clear why?

## Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

### What should the delegation keyword be named?

TODO

## Future possibilities
[future-possibilities]: #future-possibilities

TODO
