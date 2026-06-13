
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

TODO

## Drawbacks
[drawbacks]: #drawbacks

TODO

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

TODO

## Prior art
[prior-art]: #prior-art

### [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) (2015)

Delegation was first proposed in a [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406). This RFC introduces a new syntax within trait `impl` blocks, permitting a type to forward an entire trait implementation (or selected items) to a field or arbitrary expression that already implements that trait. The proposed syntax takes the forms:
- `impl Trait for Type { use expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { use expression for name_1 (, name_i)*; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a value implementing `Trait`.

Prohibited patterns: delegation of associated constants ([?]()), delegation of associated types ([?]()).<br>
Proposed extensions: renaming ([?]()), `Self` type mapping ([?]()), multiple traits ([?]()),  delegation of enums ([?]()), arbitrary parent context ([?]()).

#### Reasons for proposal rejection

_Unclear semantics._ The mechanism for callee resolution is not defined (TODO: it doesn't matter in this context btw). It's not clear what kinds of expressions are allowed in the delegation body. Underspecified `self` behavior: callee might have no receiver, might take receiver by value(`self: Self`), by reference (`self: &Self`), by mut reference(`self: &mut Self`) or even more complex types after introduction of `arbitrary_self_types` feature.

_Forward compatibility._ The RFC intentionally leaves many anticipated delegation features for future work, but there was insufficient evidence that the proposed design could be cleanly extended to those features without breaking semantics and requiring a redesign.

You can also check Boat's [summary](https://github.com/rust-lang/rfcs/pull/1406#issuecomment-269175112).

### [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) (2018)

Delegation was proposed again in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). The design was stricter to address the semantic ambiguities of the earlier proposal: it restricted delegation to fields of `self` (e.g., `self.field`).

TODO

## Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

## Future possibilities
[future-possibilities]: #future-possibilities

TODO
