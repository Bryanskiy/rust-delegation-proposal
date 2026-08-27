
- Feature Name: (fill me in with a unique ident, `fn_delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Provide a syntactic sugar to automatically forward function calls.

### Terminology and conventions

The following terminology is used in this proposal:

- _delegation item_ - a new item kind introduced by this proposal, declared with the `reuse` keyword, that generates a function or method which forwards its arguments to a specified callee.
- _target expression_ - an optional block expression that transforms the delegation item's first argument before that argument is forwarded to the resolved callee.
- _renaming_ - the ability to give the generated function a name that differs from the callee's name.
- _parent context_ - the parent item in which the delegation item appears. This can be a module (for free functions), a trait implementation, a type implementation or a trait(for associated items)
- _desugaring_ - the translation from a delegation item into regular function calls.
- TODO

The following conventions are used in this proposal:

TODO: examples, implementation notes, links

TODO: desugaring might be a subject of a change

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

This proposal revisits delegation with a more precise design.

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

Delegation items can be declared in any position where items are allowed. They are also associated items and may therefore appear in traits and implementations ([?](#why-can-delegation-items-be-declared-in-any-position-and-why-are-qualified-paths-used-for-call-disambiguation)). Like other items, delegation items may be annotated with a visibility modifier ([?](#why-may-delegation-items-be-annotated-with-a-visibility-modifier)) and may have attributes applied to them.

The delegation item has the form:
```diff
+ Delegation →
+     reuse DelegationPath ( BlockExpression | ; )
+
+ DelegationPath →
+     QualifiedPathType :: DelegationPathSegment
+   | QualifiedPathType :: { ( , DelegationPathSegment )+ ,? }
+   | QualifiedPathType :: *
+
+ DelegationPathSegment →
+     PathExprSegment ( as IDENTIFIER )?
```

Qualified paths provide an unambiguous way to identify callable items, including trait methods, trait implementation methods, inherent methods, and free functions ([?](#why-can-delegation-items-be-declared-in-any-position-and-why-are-qualified-paths-used-for-call-disambiguation)). Delegation of types and constants is not supported ([?](#why-delegation-of-types-and-constants-is-not-supported)).

> [!NOTE]
>
> Delegation to inherent methods is particularly complex to implement. From the name resolution perspective paths in Rust may be classified as follows(See [RFC 0132](https://github.com/rust-lang/rfcs/blob/master/text/0132-ufcs.md)):
> - a path to a free function (e.g., `module::func`).
> - a  reference to an associated item defined from a trait (e.g., `<Vec<T> as Clone>::clone`), where the `Self` type may also be omitted.
> - a type-relative path (e.g., `<T>::default`);
>
>

TODO: When no block is given (the `;` form), the first argument is passed through unchanged, i.e., it is effectively an alias for `{ self }`. why {}.`self` inside expression

The delegation item comes in three flavors, matching the three forms of the `DelegationPath`: [individual delegation](#individual-delegation), [list delegation](#list-delegation) and [glob delegation](#glob-delegation).

### Individual delegation

TODO

#### Function header (function qualifiers)

Function qualifiers are generated as follows:

__const:__  If the callee is a const function, the generated function is also `const`. This is necessary for the delegation to be usable in const contexts. <br>
__async__: If the callee is `async`, the generated function is also `async`. This is necessary for the delegation to be usable in async contexts. TODO: not sure about this. <br>
__unsafe:__ If the callee is `unsafe`, the generated function is also `unsafe`. <br>
__ABI:__ The generated function inherits the same ABI. <br>

TODO: coercion

### List delegation

TODO: consider how different target expressions might be applied to individual items like with use chain shortcuts.

TODO

### Glob delegation

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

- [Why can delegation items be declared in any position and why are qualified paths used for call disambiguation?](#why-can-delegation-items-be-declared-in-any-position-and-why-are-qualified-paths-used-for-call-disambiguation)
- [Why may delegation items be annotated with a visibility modifier?](#why-may-delegation-items-be-annotated-with-a-visibility-modifier)
- [Why delegation of types and constants is not supported?](#why-delegation-of-types-and-constants-is-not-supported)
- TODO: continue

#### Why can delegation items be declared in any position and why are qualified paths used for call disambiguation?

Delegation is fundamentally the forwarding of function calls. A regular function in Rust may be a trait method, a method in a trait implementation, an inherent method, or a free function. We can form different combinations based on the position of a caller and a callee:

- Delegating from a trait implementation to an implementation of the same trait.
- Delegating from a trait implementation to an implementation of another trait.
- Delegating from an inherent method to a trait implementation.
- Delegating from a trait implementation to an inherent method.
- Delegating from a free function to another free function.
- etc.

All these combinations appear in real world code via regular calls and each represents a potential target for the delegation feature. Choosing which combinations to support is a design decision driven by multiple factors: the function call resolution algorithm, the available syntax budget, the frequency of the use case and the extensibility to other cases.

Rust distinguishes between two kinds of function invocation. The first one is [method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html), which have the form `receiver.method(args...)`. They are resolved to associated methods that take a receiver argument. Resolution it that case requires additional analysis by the compiler: the receiver may be automatically dereferenced, borrowed or coerced. If more than one method is applicable the compiler emits an error. The second kind is [fully qualified calls](https://doc.rust-lang.org/reference/expressions/call-expr.html#r-expr.call.desugar) which can be used to resolve such ambiguity.

From the delegation's perspective the alternatives can be categorized as follows:

1. __Resolve the callee from the method name alone.__
[rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) suggested to use method name only to resolve the callee. This covers the most common scenario: delegating a trait implementation to another implementation of the same trait. However this syntax does not generalize naturally to other caller/callee combinations since it can lead to ambiguities in a similar way to method calls.
1. __Resolve the callee from the fully qualified path.__ This approach covers every possible caller/callee combination without ambiguity, but it requires more verbose and explicit syntax. This is the option chosen for this proposal and it can later be extended in a forward-compatible way to also support the first option.
2. __Keywords as disambiguators.__ One of the suggestion from [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) is to use keywords (`trait`/`impl`/`fn`) e.g. (`reuse trait TraitName { expression }`) to disambiguate callee. This proposal doesn't take that approach. The primary reason is that fully qualified paths already provide a uniform and well‑understood mechanism for disambiguation. Reinventing a separate keyword‑based approach(or any other alternative) would add unnecessary complexity.
3. __Analyse target expression.__ Another possibility is to use a target expression to infer the callee, i.e. the compiler would take the type of the expression and then resolve the method by name. This path is left for an alternative reflection-based language feature [(?)](#reflection).

#### Why may delegation items be annotated with a visibility modifier?

 The proposed syntax allows the visibility of a reused function to be specified independently from the visibility of the original definition. While this flexibility is desirable, it introduces a potential _semver hazard_:

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

#### Why delegation of types and constants is not supported?

Types live in the type namespace, while functions and constants live in the value namespace. A single qualified path doesn't say which namespace to pull from, so `Trait::name` is ambiguous whenever `Trait` has both an associated type and an associated fn/const called `name`.

TODO: alternatives

### Alternatives to this RFC

TODO: check Go lang

TODO: macros with link to prior art

#### Reflection

An alternative approach to delegation in Rust would be some form of compile-time reflection. Given the ability to inspect type information such as function signatures during macro expansion, delegation can be implemented entirely as a third-party library, removing the need for dedicated language support.

However, reflection is a large and complex feature that may take years to implement and stabilise. Even if it becomes available it is not clear that it would be the suitable vehicle for delegation.

Work in this direction is already being explored. See [reflection project goal](https://github.com/rust-lang/rust-project-goals/issues/406).

## Prior art
[prior-art]: #prior-art

TODO: add macro based crates

TODO: the whole point of previous RFCs analysis is to point out there were issues with forward compatibility and unclear semantics. In new proposal we solve this by better "design space exploration".

TODO: consider what to write about extensions

### [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) (2015)

Delegation was first proposed in a [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406). This RFC introduces a new syntax within trait `impl` blocks, permitting a type to forward an entire trait implementation (or selected items) to a field or arbitrary expression that already implements that trait. The proposed syntax takes the forms:
- `impl Trait for Type { use expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { use expression for name_1 (, name_i)*; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a value implementing `Trait`.

#### Main reasons for proposal rejection

_Unclear semantics._ It's not clear what kinds of expressions are allowed in the delegation body. Underspecified `self` behavior: callee might have no receiver, might take receiver by value(`self: Self`), by reference (`self: &Self`), by mut reference(`self: &mut Self`) or even more complex types after introduction of `arbitrary_self_types` feature. The mechanism for desugaring is not defined.

_Forward compatibility._ The RFC intentionally leaves many features for future work, but there was insufficient evidence that the proposed design could be clearly extended to those features without breaking semantics and requiring a redesign.

### [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) (2018)

Delegation was proposed again in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). The design was stricter to address the semantics ambiguities of the earlier proposal. The syntax takes the forms:
- `impl Trait for Type { delegate * to expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { delegate fn name_1 (, fn name_i)* to expression; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a field of `self` (e.g., `self.field`) and `typeof(expression)` implements `Trait`.

Delegation is allowed only for methods that take a receiver by value, by reference or by mutable reference. Other cases are left for future extensions. Proposed desugaring scheme translates delegation item into [method call](https://doc.rust-lang.org/reference/expressions/method-call-expr.html).

#### Main reasons for proposal rejection

The second proposal was [postponed](https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011) due to the lang team bandwidth. Additionally, forward compatibility concerns were never fully addressed.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

### What keyword should be used?

The draft uses `reuse`, but other options like `delegate` or `forward` could be considered. The keyword should not conflict with existing identifiers and should be intuitive.

TODO: `use` is one of the alternatives, check concerns from first proposal.

## Future possibilities
[future-possibilities]: #future-possibilities

TODO
