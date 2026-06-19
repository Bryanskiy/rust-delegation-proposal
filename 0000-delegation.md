
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

Delegation items can be declared in any position where items are allowed. They are also associated items and may therefore appear in traits and implementations.

TODO

## Drawbacks
[drawbacks]: #drawbacks

TODO

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

TODO

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

The second proposal was postponed due to the lang team bandwidth. ([?](https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011)).

TODO: Mention that forward compatibility concern wasn't addressed. Should we provide example or it's clear why?

## Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

### What should the delegation keyword be named?

TODO

## Future possibilities
[future-possibilities]: #future-possibilities

TODO
