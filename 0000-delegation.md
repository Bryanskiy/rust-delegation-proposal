
- Feature Name: (fill me in with a unique ident, `fn_delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Provide a syntactic sugar to automatically forward function calls.

## Terminology and conventions

The following terminology is frequently used in this proposal:

- _delegation item_ - a new item kind introduced by this proposal, declared with the `reuse` keyword, that generates a function or method which forwards its arguments to a specified callee.
- _target expression_ - an optional block expression that transforms the delegation item's first argument before that argument is forwarded to the resolved callee.
- _renaming_ - the ability to give the generated function a name that differs from the callee's name.
- _parent context_ - the parent item in which the delegation item appears. This can be a module (for free functions), a trait implementation, a type implementation or a trait(for associated items).
- _desugaring_ - the translation from a delegation item into regular function calls.
- TODO

The following conventions are used in this proposal:

TODO: links to rational/external/other sections </br>
TODO: notes to implementation experience, other notes </br>
TODO: examples </br>
TODO: desugaring might be a subject of a change </br>
TODO: note that doc format was taken from another rfc/create something else

## Motivation

Rust does not provide the kind of data inheritance common in object-oriented languages where a derived type automatically inherits methods from a base type. Instead Rust typically expresses this pattern through _composition_: the "base" type is embedded inside the "derived" type as a field (possibly nested) or another form of subobject. With composition methods that would be inherited automatically in other languages must instead be implemented manually often with the help of macros. Although these forwarding implementations are usually trivial they impose a practical cost in terms of verbosity and readability.

Consider a common pattern found throughout real Rust codebases:

```rust
// library/alloc/src/collections/btree/set.rs

impl<T: Hash, A: Allocator + Clone> Hash for BTreeSet<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.map.hash(state)
    }
}
```

The implementation does not introduce new behavior. It simply forwards a method call to a field. In practice the required repetition may even discourage the use of newtypes despite their advantages for type safety and abstraction. This situation highlights a gap in Rust’s ergonomics. While Rust provides powerful mechanisms for defining abstractions through traits and generics it offers comparatively little support for reusing existing behavior.

This limitation has long been recognized by the Rust community. TODO: link to prior work

This proposal revisits delegation.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Delegation items resemble `use` declarations.

```rust
// Import
#[attrs]
pub(vis) use prefix::{a, b, c as d};

// Delegation item
#[attrs]
pub(vis) reuse prefix::{a, b, c as d} { target_expr }
```

TODO: example with min, ranges

TODO

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This proposal introduces a new [item kind](https://doc.rust-lang.org/reference/items.html), the delegation item:

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

Delegation items can be declared in any position where items are allowed. They are also associated items and may therefore appear in traits and implementations ([?](#why-can-delegation-items-be-declared-in-any-position)). Like other items, delegation items may be annotated with a visibility modifier ([?](#why-may-delegation-items-be-annotated-with-a-visibility-modifier)) and may have attributes applied to them ([?](#why-are-attributes-manually-added-instead-of-being-inherited-from-the-callee)).

The delegation item has the form:
```diff
+ Delegation →
+     reuse DelegationPath ( BlockExpression | ; )
+
+ DelegationPath →
+     QualifiedPathType :: DelegationPathSegment
+   | QualifiedPathType :: { ( DelegationPathSegment )+ ,? }
+   | QualifiedPathType :: *
+
+ DelegationPathSegment →
+     PathExprSegment ( as IDENTIFIER )?
```

A delegation item starts with the `reuse` keyword and consists of a [fully qualified path](#qualified-paths-and-name-resolution), optionally followed by a [target expression](#target-expression). It comes in three flavors, matching the three forms of `DelegationPath`: [individual delegation](#individual-delegation), [list delegation](#list-delegation) and [glob delegation](#glob-delegation).

_See the following sections for rationale/alternatives_:

- [Why can delegation items be declared in any position?](#why-can-delegation-items-be-declared-in-any-position)
- [Why may delegation items be annotated with a visibility modifier?](#why-may-delegation-items-be-annotated-with-a-visibility-modifier)
- [Why are attributes manually added instead of being inherited from the callee?](#why-are-attributes-manually-added-instead-of-being-inherited-from-the-callee)

_See the following sections for unresolved questions_:

- [What keyword should be used?](#what-keyword-should-be-used)

### Qualified paths and name resolution

Qualified paths provide an unambiguous way to identify callable items, including trait methods, trait implementation methods, inherent methods, and free functions ([?](#why-are-qualified-paths-used-for-call-disambiguation), [?](#why-is-self-type-allowed-in-qualified-paths)). Delegation of types and constants is not supported ([?](#why-is-delegation-of-types-and-constants-not-supported)).

> [!NOTE]
>
> Delegation to inherent methods is particularly complex to implement. From the name resolution perspective paths in Rust may be classified as follows(See [RFC 0132](https://github.com/rust-lang/rfcs/blob/master/text/0132-ufcs.md)):
> - a path to a free function (e.g., `module::func`).
> - a  reference to an associated item defined from a trait (e.g., `<Vec<T> as Clone>::clone`), where the `Self` type may also be omitted.
> - a type-relative path (e.g., `<T>::default`);
>
> Lowering a delegation item into a real function requires knowing the callee's signature including: generics, number of arguments, whether and how it takes `self` argument. With this information a _compatible_ signature can be synthesized for the new item. Paths in the first two categories can be resolved early enough to expose that information. Type-relative paths generally cannot: their resolution is not known until type-checking, by which point the delegation item's signature is already needed.
>
> TODO: continue

_See the following sections for rationale/alternatives_:

- [Why are qualified paths used for call disambiguation](#why-are-qualified-paths-used-for-call-disambiguation)
- [Why is `Self` type allowed in qualified paths?](#why-is-self-type-allowed-in-qualified-paths)
- [Why is delegation of types and constants not supported?](#why-is-delegation-of-types-and-constants-not-supported)

### Target expression

TODO: When no block is given (the `;` form), the first argument is passed through unchanged, i.e., it is effectively an alias for `{ self }`. why {}.`self` inside expression

### Individual delegation

Individual delegation is the simplest form: it declares exactly one new item, forwarding to exactly one callee named by `DelegationPath`.

Function qualifiers are inherited unchanged from the callee. None of these qualifiers can be added, removed, or overridden at the delegation site ([?](#why-are-function-qualifiers-inherited-unchanged-from-the-callee)).

TODO

Delegation of variadic functions is not supported ([?](#why-is-delegation-of-variadic-functions-not-supported)).

TODO

callee might have no receiver, might take receiver by value(`self: Self`), by reference (`self: &Self`), by mut reference(`self: &mut Self`) or even more complex types after introduction of `arbitrary_self_types` feature.

TODO

_See the following sections for rationale/alternatives_:

- [Why are function qualifiers inherited unchanged from the callee?](#why-are-function-qualifiers-inherited-unchanged-from-the-callee)
- [Why is delegation of variadic functions not supported?](#why-is-delegation-of-variadic-functions-not-supported)

### List delegation

List delegation declares several items at once from a shared path prefix. This desugars to one individual delegation item per name.

TODO: consider how different target expressions might be applied to individual items like with use chain shortcuts.

TODO

### Glob delegation

Glob delegation delegates every method of a trait in one go. It's only permitted inside a trait implementations.

TODO

## Drawbacks
[drawbacks]: #drawbacks

1. __Coverage__: Many cases of delegation require more than simple forwarding (e.g., transforming arguments or return values). This feature only handles the simplest case leaving complex transformations to manual coding or macros. This might limit its usefulness.
2. __Potential redundancy__: The delegation feature could be implemented entirely via compile‑time reflection [(?)](#reflection) (if and when that becomes available).
3. __Increased language complexity__: duh

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- [Design decisions outlined in this RFC](#design-decisions-outlined-in-this-rfc)
- [Alternatives to this RFC](#alternatives-to-this-rfc)
- TODO: we have some statistics and we can add it here

### Design decisions outlined in this RFC

#### Why can delegation items be declared in any position?

Delegation is fundamentally the forwarding of function calls. A regular function in Rust may be a trait method, a method in a trait implementation, an inherent method, or a free function. We can form different combinations based on the position of a caller and a callee:

- Delegating from a trait implementation to an implementation of the same trait.
- Delegating from a trait implementation to an implementation of another trait.
- Delegating from an inherent method to a trait implementation.
- Delegating from a trait implementation to an inherent method.
- Delegating from a free function to another free function.
- etc.

All these combinations appear in real world code via regular calls and each represents a potential target for the delegation feature. Choosing which combinations to support is a design decision driven by multiple factors: the function call resolution algorithm, the available syntax budget, the frequency of the use case and the extensibility to other cases.

In this proposal we support every combination rather than special-casing the most common ones, because the chosen name resolution scheme permits to resolve every kind of callee, and every combination lowers through the same desugaring scheme once the path is resolved.

_See the following sections for rational/alternatives_:

- [why are qualified paths used for call disambiguation?](#why-are-qualified-paths-used-for-call-disambiguation)

↩ [reference-level explanation](#reference-level-explanation)

#### why are qualified paths used for call disambiguation?

__Note__: Rust distinguishes between two kinds of function invocation. The first one is [method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html), which have the form `receiver.method(args...)`. They are resolved to associated methods that take a receiver argument. Resolution it that case requires additional analysis by the compiler: the receiver may be automatically dereferenced, borrowed or coerced. If more than one method is applicable the compiler emits an error. The second kind is [fully qualified calls](https://doc.rust-lang.org/reference/expressions/call-expr.html#r-expr.call.desugar) which can be used to resolve such ambiguity.

From the delegation's perspective the alternatives can be categorized as follows:

1. Resolve the callee from the method name alone.

    [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) suggested to use method name only to resolve the callee. This covers the most common scenario: delegating a trait implementation to another implementation of the same trait. However this syntax does not generalize naturally to other caller/callee combinations since it can lead to ambiguities in a similar way to method calls.

    __Note:__ One of the possibilities to infer the callee is to analyse target expression, i.e. the compiler would take the type of the expression(e.g. `typeof(expression)`) and then resolve the method by name. This approach raises open questions of its own: how ambiguities between multiple equally-named candidates would be resolved, and how broad a range of cases such inference could realistically support. For these reasons, it is left to a possible alternative reflection-based language feature [(?)](#reflection).

2. Resolve the callee from the fully qualified path.

   This approach covers every possible caller/callee combination without ambiguity, but it requires more verbose and explicit syntax.

3. Use keywords as disambiguators.

    One of the suggestion from [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) is to use keywords (`trait`/`impl`/`fn`) e.g. (`reuse trait TraitName { expression }`) to disambiguate callee. However, this approach cannot distinguish between multiple generic implementations of the same trait.


The second option has been chosen for this proposal. The first reason is that fully qualified paths already provide a uniform and well‑understood mechanism for disambiguation. Reinventing a separate keyword‑based approach(or any other alternative) would add unnecessary complexity. The second reason is that the first option has already been proposed twice, in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). Rather than attempt the same approach a third time, this proposal comes at the problem from a different angle: because every callee is already reachable through a fully qualified path, name-based resolution can be reintroduced later as pure syntactic sugar layered on top of that mechanism. That keeps the door open to the first option in a forward-compatible way.

_See the following sections for future possibilities_:

- [name-based resolution as sugar](#name-based-resolution-as-sugar)

↩ [qualified paths and name resolution](#qualified-paths-and-name-resolution)

#### Why is `Self` type allowed in qualified paths?

TODO: methods without receiver </br>
TODO: example with `Iterator` and `UnordItems`

↩ [qualified paths and name resolution](#qualified-paths-and-name-resolution)

#### Why may delegation items be annotated with a visibility modifier?

The ability to specify a reused function's visibility independently of the original's is a welcome flexibility, yet it introduces a potential semver hazard:

```rust
fn foo<T: Copy>(x: T) { /* impl */ }
pub reuse foo as bar;
```

If the signature of `foo` changes, the generated function `bar` changes accordingly. In regular Rust code, such a signature change would cause a type error at every call site, forcing the author to update callers. With `reuse`, however, the change propagates silently. As a result, modifications intended to be internal may accidentally become breaking changes for downstream crates.

Taking this into consideration, several design choices are possible:

1. The visibility of the generated function is taken solely from the callee.
2.  The visibility of the generated function cannot exceed the visibility of the reused function. In other words, delegation may only preserve or reduce visibility, never increase it.
3. Explicit visibility control by the user.

We prefer to leave all control to the user while also adding a deny-by-default lint that prevents a generated function from having greater visibility than the callee. The visibility handling can be refined prior to stabilization based on experience and feedback and does not block this proposal.

_See the following sections for unresolved questions_:

- [Should the visibility of the delegation item be restricted?](#should-the-visibility-of-the-delegation-item-be-restricted)

↩ [reference-level explanation](#reference-level-explanation)

#### Why are attributes manually added instead of being inherited from the callee?

Delegation item is a distinct item that may deliberately want different behavior than its callee.

Attributes may affect diagnostics, linking, documentation, or the item's public API contract and auto-inheriting attributes would also mean a delegation item's behavior could change silently whenever the callee's attributes change, with no corresponding edit at the delegation site.

> [!NOTE]
>
> The compiler may automatically add some attributes to a function whenever doing so doesn't change observable behavior. For instance, the current implementation adds the `#[inline]` attribute: inlining is purely an optimisation, so it can't change what the delegation item means and it keeps a  forwarding wrapper as close to zero-cost abstraction as writing the call by hand.

↩ [reference-level explanation](#reference-level-explanation)

#### Why is delegation of types and constants not supported?

Types live in the type namespace, while functions and constants live in the value namespace. A single qualified path doesn't say which namespace to pull from, so `Trait::name` is ambiguous whenever `Trait` has both an associated type and an associated fn/const called `name`.

TODO: alternatives

#### Why are function qualifiers inherited unchanged from the callee?


The function header comprises qualifiers such as `const`, `async`, `unsafe`, `extern "ABI"`.

- If the callee is a const function, the generated function is also `const`. This is necessary for the delegation to be usable in const contexts. <br>
- If the callee is `async`, the generated function is also `async`. This is necessary for the delegation to be usable in async contexts. <br>
- If the callee is `unsafe`, the generated function is also `unsafe`. Delegation merely forwards the call and cannot verify the safety contract required by the callee. Therefore, the same safety obligations must be imposed on caller. <br>
- The generated function inherits the same ABI. <br>

Programmer who wants a different behavior can still write a wrapper by hand.

TODO: fn ptr coercion

↩ [individual delegation](#individual-delegation)

#### Why is delegation of variadic functions not supported?

TODO: find github issue

↩ [individual delegation](#individual-delegation)

### Alternatives to this RFC

TODO: check Go lang

TODO: macros with link to prior art

#### Inheritance

TODO

#### Reflection

An alternative approach to delegation in Rust would be some form of compile-time reflection. Given the ability to inspect type information such as function signatures during macro expansion, delegation can be implemented entirely as a third-party library, removing the need for dedicated language support.

However, reflection is a large and complex feature that may take years to implement and stabilise. Even if it becomes available it is not clear that it would be the suitable vehicle for delegation.

Work in this direction is already being explored. See [reflection project goal](https://github.com/rust-lang/rust-project-goals/issues/406).

## Prior art
[prior-art]: #prior-art

TODO: add macro based crates

TODO: consider what to write about extensions

### [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) (2015)

Delegation was first proposed in a [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406). This RFC introduces a new syntax within trait `impl` blocks, permitting a type to forward an entire trait implementation (or selected items) to a field or arbitrary expression that already implements that trait. The proposed syntax takes the forms:
- `impl Trait for Type { use expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { use expression for name_1 (, name_i)*; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `typeof(expression)` implements `Trait`.

#### Main reasons for proposal rejection

_Unclear semantics._ It's not clear what kinds of expressions are allowed in the delegation body. Underspecified `self` behavior. The mechanism for desugaring is not defined. Also see [comment](https://github.com/rust-lang/rfcs/pull/1406#issuecomment-269175112).

_Forward compatibility._ The RFC intentionally leaves many features for future work, but there was insufficient evidence that the proposed design could be clearly extended to those features without breaking semantics and requiring a redesign.

### [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) (2018)

Delegation was proposed again in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). The design was stricter to address the semantics ambiguities of the earlier proposal. The syntax takes the forms:
- `impl Trait for Type { delegate * to expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { delegate fn name_1 (, fn name_i)* to expression; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a field of `self` (e.g., `self.field`) and `typeof(expression)` implements `Trait`.

Delegation is allowed only for methods that take a receiver by value, by reference or by mutable reference. Proposed desugaring scheme translates delegation item into [method call](https://doc.rust-lang.org/reference/expressions/method-call-expr.html).

#### Main reasons for proposal rejection

The second proposal was [postponed](https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011) due to the lang team bandwidth. Additionally, forward compatibility concerns were never fully addressed.

### [crates.io/delegate](https://crates.io/crates/delegate)

One of the most used crate for delegation. It implements the `delegate!` declarative macro, which delegates method calls to selected expressions.

__Strengths__:

- The main advantage is the variety of transformations of the signature and the body of the generated method.

__Weaknesses__:

TODO:

### [crates.io/ambassador](http://crates.io/crates/ambassador)

The second most popular crate for delegation. in contrast with [delegate](https://crates.io/crates/delegate), procedural macros are used, not declarative ones.

Strengths:

TODO:

Weaknesses:

TODO:

### [rfcs2375](https://github.com/rust-lang/rfcs/pull/2375) (2018)

TODO: this is not delegation, but the use case can be covered by delegation.

### [rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591) (2024)

TODO: this is not delegation, but the use case can be covered by delegation.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

### Should the visibility of the delegation item be restricted?

TODO

↩ [Why may delegation items be annotated with a visibility modifier?](#why-may-delegation-items-be-annotated-with-a-visibility-modifier)

### What keyword should be used?

The draft uses `reuse`, but other options like `delegate` or `forward` could be considered. The keyword should not conflict with existing identifiers and should be intuitive.

TODO: `use` is one of the alternatives, check concerns from first proposal.

↩ [reference-level explanation](#reference-level-explanation)

## Future possibilities
[future-possibilities]: #future-possibilities

TODO

### Name-based resolution as sugar

A shorter syntax that infers the callee from a bare method name could be layered on top of fully qualified paths.
