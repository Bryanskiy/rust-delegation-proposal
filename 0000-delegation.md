
- Feature Name: (fill me in with a unique ident, `fn_delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC proposes a design for _delegation_: syntactic sugar for ergonomically forwarding function calls.

## Terminology

The following terminology is frequently used in this proposal:

- _delegation item_ - a new item kind introduced by this proposal, declared with the `reuse` keyword, that generates a function or method which forwards its arguments to a specified callee.
- _target expression_ - an optional block expression that transforms the delegation item's first argument before that argument is forwarded to the resolved callee.
- _renaming_ - the ability to give the generated function a name that differs from the callee's name.
- _parent context_ - the parent item in which the delegation item appears. This can be a module (for free functions), a trait implementation, a type implementation or a trait(for associated items).
- _desugaring_ - the translation from a delegation item into regular function calls.
- _delegation pattern_ - a piece of code that can potentially be rewritten using a delegation item.
- _delegation resolution_ - a function from which the signature is copied during desugaring.

## Implementation experience

This RFC draws on the experimental implementation tracked in [rust-lang/rust#118212](https://github.com/rust-lang/rust/issues/118212).

Many of the examples in this proposal can be tried on nightly Rust. However the implementation is still incomplete, contains some questionable design decisions and may not work correctly in all cases, particularly for delegation of inherent methods and in generic contexts. These limitations are discussed throughout the proposal.

TODO: 2 section:  we have parts that we are sure, we have parts that we implemented in some way, but very questionable. Somehow tell about this.

## How to read this RFC

This RFC is quite long, and a few kinds of cross-reference recur throughout it, so it's worth spelling out the convention up front:

- A ([?](#anchor)) link points to a rationale subsection under Rationale and alternatives explaining why a design decision was made the way it was. These are asides: skipping them costs nothing for understanding the feature itself, only the reasoning behind one specific choice.
- A [_text in italics_](#anchor) link points to another section of the RFC: material the current paragraph depends on.
- A plain [text](url) link points outside this RFC such as a pull request, issue, comment, crate, or page of the Rust reference.

TODO: Check links</br>
TODO: notes to implementation experience, other notes </br>
TODO: examples </br>
TODO: note that doc format was taken from another rfc/create something else

## Motivation

Rust [deliberately](https://doc.rust-lang.org/book/ch18-01-what-is-oo.html#inheritance-as-a-type-system-and-as-code-sharing) does not provide the kind of data inheritance common in object-oriented languages where a derived type automatically inherits methods from a base type. Instead Rust typically expresses this pattern through composition: the "base" type is embedded inside the "derived" type as a field (possibly nested) or another form of subobject. With composition methods that would be inherited automatically in other languages must instead be implemented manually often with the help of macros. Although these forwarding implementations are usually trivial they impose a practical cost in terms of verbosity and readability.

Consider a common pattern found throughout real Rust codebases:

```rust
// library/alloc/src/collections/btree/set.rs

impl<T: Hash, A: Allocator + Clone> Hash for BTreeSet<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.map.hash(state)
    }
}
```

The is simply forwards a method call to a field. In practice the required repetition may even discourage the use of newtypes despite their advantages for type safety and abstraction. This situation highlights a gap in Rust’s ergonomics. While Rust provides powerful mechanisms for defining abstractions through traits and generics it offers comparatively little support for reusing existing behavior.

This limitation has long been recognized by the Rust community: it has motivated two prior RFCs ([#1406](https://github.com/rust-lang/rfcs/pull/1406), [#2393](https://github.com/rust-lang/rfcs/pull/2393)), a multiple discussions, and several macro crates ([delegate](https://crates.io/crates/delegate) and [ambassador](https://crates.io/crates/ambassador) are most popular amongst them). See [_Prior art_](#prior-art) for a discussion of these efforts.

This proposal revisits delegation.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TODO: continue developing example with with more advanced features.

Suppose you're writing a `BTreeSet<T>` type as a wrapper around `BTreeMap<T, ()>` which is, incidentally, close to how the standard library's own `BTreeSet` is actually built (real `BTreeSet` also carries an allocator parameter, elided here for simplicity).

```rust
pub struct BTreeSet<T> {
    map: BTreeMap<T, ()>,
}
```

The [motivation](#motivation) section already showed how the standard library forwards `Hash` by hand. With a delegation, the same implementation is:

```rust
impl<T: Hash> Hash for BTreeSet<T> {
    reuse Hash::hash { self.map }
}
```

`reuse` item is a new delegation item. `Hash::hash` is the callee, and `{ self.map }` is the target expression: a small block which replaces the callee's first argument.

TODO: The compiler needs an explicit hint such as `Hash::hash` rather than just `hash`, because callee might differ. Check `Default` trait impl.

### Other parent context

Delegation isn't limited to trait methods. `BTreeMap` implements `contains_key` as an inherent method, and reuse can forward it just as easily:

```rust
impl<T: Ord> BTreeSet<T> {
    reuse BTreeMap::<T, ()>::contains_key { self.map }
}
```

TODO: continue

### Renaming a delegated method

Sometimes the callee's name isn't the name you want on your own type. `contains_key` reads naturally on a map, but for a set `contains` is clearer. Adding `as new_name` after the callee renames the generated method:

```rust
impl<T> BTreeSet<T> {
    reuse BTreeMap::<T, ()>::contains_key as contains { self.map }
}
```

### Delegating several methods at once

Listing out `is_empty`, `clear` and `len` as three separate reuse items is still three lines whose only real difference is the method name. List delegation collapses them into one:

```rust
impl<T> BTreeSet<T> {
    reuse BTreeMap<T, ()>::{len, is_empty, clear, capacity} { self.map }
}
```

Each generated method gets the receiver its callee needs, not a receiver you have to spell out yourself: `clear` needs to mutate the map, so the method this generates takes `&mut self`, while `len` and `is_empty` only need to read it, so those take the shared reference `&self`. The target expression `{ self.map }` is the same in all 3 cases.

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

Delegation items can be declared in any context where functions with bodies are permitted by the semantic rules. For example, delegation items cannot be declared inside an `extern` block. They are also associated items and may therefore appear in traits and implementations ([?](#why-can-delegation-items-be-declared-in-any-position)). Like other items, delegation items may be annotated with a visibility modifier ([?](#why-is-visibility-manually-added-instead-of-being-copied-from-the-callee)) and may have attributes applied to them ([?](#why-are-attributes-manually-added-instead-of-being-copied-from-the-callee)).

The delegation item has the form:
```diff
+ Delegation →
+     reuse DelegationPath ( BlockExpression | ; )
+
+ DelegationPath →
+     Path :: DelegationPathSegment
+   | Path :: { ( DelegationPathSegment )+ ,? }
+   | Path :: *
+
+ DelegationPathSegment →
+     PathExprSegment ( as IDENTIFIER )?
```

A delegation item starts with the `reuse` keyword ([?](#what-keyword-should-be-used)) and consists of
- a path, which may be either simple or qualified. See the following [_Paths and name resolution_](#paths-and-name-resolution) section for a discussion of the rules and implementation details associated with name resolution.
- an optional block expression. See the following [_Target expression_](#target-expression) section for detailed discussion of the rules associated with it.

Delegation item comes in three flavors: individual delegation, list delegation ([?](#why-is-list-delegation-supported)) and glob delegation ([?](#why-is-glob-delegation-supported)). The optional `as IDENTIFIER` allows to expose the delegated function under a different name ([?](#why-is-renaming-supported)).

Delegation of types and constants is not currently supported ([?](#support-delegating-types-and-consts)). Delegation item doesn't provide syntax for introducing its own generics ([?](#why-doesnt-a-delegation-item-provide-syntax-for-introducing-its-own-generics)). Delegation item doesn't provide syntax for arguments or return value transformations ([?](#why-doesnt-a-delegation-item-provide-syntax-for-arguments-or-return-value-transformations)).

_See the following sections for rationale/alternatives_:

- [Why can delegation items be declared in any position?](#why-can-delegation-items-be-declared-in-any-position)
- [Why is visibility manually added instead of being copied from the callee?](#why-is-visibility-manually-added-instead-of-being-copied-from-the-callee)
- [Why are attributes manually added instead of being copied from the callee?](#why-are-attributes-manually-added-instead-of-being-copied-from-the-callee)
- [Why is list delegation supported?](#why-is-list-delegation-supported)
- [Why is glob delegation supported?](#why-is-glob-delegation-supported)
- [Why is renaming supported?](#why-is-renaming-supported)
- [Why doesn't a delegation item provide syntax for introducing its own generics?](#why-doesnt-a-delegation-item-provide-syntax-for-introducing-its-own-generics)
- [Why doesn't a delegation item provide syntax for arguments or return value transformations?](#why-doesnt-a-delegation-item-provide-syntax-for-arguments-or-return-value-transformations)

_See the following sections for unresolved questions_:

- [Should the visibility of the delegation item be restricted?](#should-the-visibility-of-the-delegation-item-be-restricted)
- [Which attributes should be added by default?](#which-attributes-should-be-added-by-default)
- [What keyword should be used?](#what-keyword-should-be-used)

_See the following sections for future possibilities_:

- [Support delegating types and consts](#support-delegating-types-and-consts)

### Desugaring of individual delegation

Individual delegation is the simplest case: it declares exactly one new item that forwards to exactly one callee named by a path. We name the function from which the delegated item's information is copied the delegation resolution. For delegation declared in a trait implementation, the delegation resolution is the corresponding trait method ([?](#why-is-the-delegation-resolution-the-trait-being-implemented-in-trait-implementations)). In all other cases, it is the item resolved by the path. (See [_Paths and name resolution_](#paths-and-name-resolution) for details on how the path is resolved).

> [!NOTE]
>
> Desugaring happens mainly during [AST lowering](https://rustc-dev-guide.rust-lang.org/hir/lowering.html). This is because once [HIR](https://rustc-dev-guide.rust-lang.org/hir.html) construction is complete the crate becomes immutable and code modification is no longer possible at that stage.

The generated function body for an individual delegation have the form:

```
#[attrs]
pub(vis) FunctionQualifiers fn name<GenericParams>(..., argN: ArgN, ...) FunctionReturnType
WhereClause
{
    #![attrs]
    target_expr_stmt_1;
    ....
    target_expr_stmt_n;
    path(..., ADJ(target_expr_operand(argN)), ...)
}
```

- Outer attributes (`#[attrs]`) are exactly those specified by the user at the delegation site, if any, plus default attributes.
- Inner attributes (`#![attrs]`) are exactly those specified by the user inside target expression, if any. TODO: or, if prohibited, move to rationale
- Visibility `(pub(vis))` is exactly as specified by the user at the delegation site.
- Function qualifiers(`FunctionQualifiers`) are copied unchanged from the delegation resolution. None of these qualifiers can be overridden ([?](#why-are-function-qualifiers-copied-unchanged)).
- The function name (`name`) is the identifier following `as` keyword, or, if no `as` clause is specified, the final segment of `path`.
- Function arguments (e.g. `argN: ArgN`) are copied from the delegation resolution:
  - Generic parameters appearing in the function arguments are remapped as described in [_Generics remapping_](#Generics-remapping).
  - TODO: depending on `Self` type
- Return type (`FunctionReturnType`) is copied from the delegation resolution:
  - Generic parameters appearing in the return type are remapped as described in [_Generics remapping_](#Generics-remapping).
  - TODO: depending on `Self` type
- Generic parameters(`GenericParams`) and where clause(`WhereClause`) are copied from the delegation resolution and remapped as described in [_Generics remapping_](#Generics-remapping).
- The target expression consists of a list of statements (`target_expr_stmt_i`) and a final optional expression(`target_expr_operand`). In the generated function body, the statements come first ([?](#why-are-statements-not-passed-to-the-call)), followed by the function forwarding call. The arguments to which the `target_expr_operand` is applied along with other related rules are specified in the [_Target expression_](#target-expression) section. Usually, the `target_expr_operand` is applied to the method receiver.
- `ADJ` denotes the same adjustments as for an ordinary [method call](https://doc.rust-lang.org/reference/expressions/method-call-expr.html) receiver: a sequence of autoderefs, an optional autoref and coercions. The difference is that the callee has already been resolved through the path, so these adjustments are not needed for name resolution. Instead, they are applied to the arguments to make it match the callee's signature (See [_Glob delegation_](#glob-delegation) and [_List delegation_](#list-delegation) for rationale).
- The path (`path`) is exactly as specified by the user, except that the delegation resolution's own generic parameters are substituted as arguments to the final segment ([?](#why-are-the-delegation-resolutions-own-generic-parameters-substituted-as-arguments-to-the-final-segment)).
- TODO: return value transformations

_See the following sections for rationale/alternatives_:

- [Why is the delegation resolution the trait being implemented in trait implementations?](#why-is-the-delegation-resolution-the-trait-being-implemented-in-trait-implementations)
- [Why are function qualifiers copied unchanged?](#why-are-function-qualifiers-copied-unchanged)
- [Why are the delegation resolution's own generic parameters substituted as arguments to the final segment?](#why-are-the-delegation-resolutions-own-generic-parameters-substituted-as-arguments-to-the-final-segment)
- [Why are statements not passed to the call?](#why-are-statements-not-passed-to-the-call)

### Paths and name resolution

Paths provide an unambiguous way to identify callable items, including trait methods, trait implementation methods, inherent methods and free functions ([?](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)). They can also refer to other delegation items. If a cycle is encountered in the chain of recursive delegations, an error is reported.

> [!NOTE]
>
> Delegation to inherent methods is particularly complex to implement. From the name resolution perspective paths in Rust may be classified as follows:
> - a path to a free function (e.g., `module::func`).
> - a  reference to an associated item defined from a trait (e.g., `<Vec<T> as Clone>::clone`), where the `Self` type may also be omitted.
> - a type-relative path (e.g., `<T>::default`);
>
> Lowering a delegation item into a real function requires knowing the callee's signature including: generics, number of arguments, whether and how it takes `self` argument. With this information a _compatible_ signature can be synthesized for the new item. Paths in the first two categories can be resolved early enough to expose that information. Type-relative paths generally cannot: their resolution is not known until type-checking, by which point the delegation item's signature is already needed.
>
> TODO: continue

TODO(move this): callee might have no receiver, might take receiver by value(`self: Self`), by reference (`self: &Self`), by mut reference(`self: &mut Self`) or even more complex types after introduction of `arbitrary_self_types` feature.

TODO(move this): Delegation of variadic functions is not supported ([?](#why-is-delegation-of-variadic-functions-not-supported)).

_See the following sections for rationale/alternatives_:

- [Why are qualified paths used for call disambiguation](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)
- [Why is delegation of variadic functions not supported?](#why-is-delegation-of-variadic-functions-not-supported)

### Generics remapping

As mentioned earlier, a delegation item does not introduce its own generic parameters. Instead, they are copied from the delegation resolution. However, we need to remap the generics so that the copied signature and where-clauses remain semantically equivalent to the delegation resolution ([?](#why-is-generic-parameter-remapping-needed)).

The following procedure is used for remapping:

1. TODO: substitution
2. If any undefined generic parameters remain in the signature or where-clauses after substitution, report an error ([?](#what-happens-if-undefined-generic-parameters-remain-after-substitution)).
3. Generated parameters are renamed to avoid colliding with generic parameters already in scope ([?](#why-are-generated-generic-parameters-renamed)).

_See the following sections for rationale/alternatives_:

- [Why is generic parameter remapping needed?](#why-is-generic-parameter-remapping-needed)
- [What happens if undefined generic parameters remain after substitution?](#what-happens-if-undefined-generic-parameters-remain-after-substitution)
- [Why are generated generic parameters renamed?](#why-are-generated-generic-parameters-renamed)

### Target expression

The target expression is an optional [block expression](https://doc.rust-lang.org/beta/reference/expressions/block-expr.html) ([?](#why-is-the-target-expression-a-block-expression)) that transforms the delegation item's first argument before that argument is forwarded to the resolved callee. When no block is given the first argument is passed through unchanged ([?](#why-can-the-block-expression-be-omitted)). There are no restrictions on the expressions that can be used inside the target expression ([?](#why-target-expression-is-not-restricted)).


Inside that block, `self` refers to TODO <br>
TODO: `self` only in the final expression? Prohibited in statements.

_See the following sections for rational/alternatives:_

- [Why is the target expression a block expression?](#why-is-the-target-expression-a-block-expression)
- [Why can the block expression be omitted?](#why-can-the-block-expression-be-omitted)
- [Why target expression is not restricted?](#why-target-expression-is-not-restricted)

### List delegation

List delegation declares several items at once from a shared path prefix. This desugars to one individual delegation item per name.

TODO

### Glob delegation

Glob delegation delegates every method of a trait in one go. It's only permitted inside a trait implementations.

TODO: how it works with defaults </br>
TODO: `reuse impl Trait` + how it works with override </br>

### When things go wrong

TODO: diagnostics <br>
TODO: problems with inherence

## Drawbacks
[drawbacks]: #drawbacks

1. Many cases of delegation require more than simple forwarding (e.g., transforming arguments or return values). This feature only handles the simplest case leaving complex transformations to manual coding or macros. This might limit its usefulness.
2. The delegation feature could potentially be implemented as third-partly library with compile‑time [reflection](#reflection) (if and when that becomes available).

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- [Design guiding principles](#design-guiding-principles)
- [Design decisions outlined in this RFC](#design-decisions-outlined-in-this-rfc)
- [Alternatives to this RFC](#alternatives-to-this-rfc)
- TODO: we have some statistics and we can add it here

### Design guiding principles

A recurring question throughout this RFC is whether a particular delegation pattern should be supported. We want to formulate several guiding principles that are used to rationalize individual design decisions.

### Rule №1: stay in syntax budget

We should fit delegation item into some syntax that is no more complex than `use` imports.

```rust
// Import item
#[attrs]
pub(vis) use prefix::{a, b, c as d};

// Delegation item
#[attrs]
pub(vis) reuse prefix::{a, b, c as d} { target_expr }
```

The motivation here is to avoid more complex features such as argument or return-value transformations, which would require pre- or post-processing closures. In such cases, the delegation item becomes less readable and more akin to a full function implementation. These transformations can instead be written manually or expressed using a macro (See [_Prior art_](#prior-art)).

### Rule №2: prefer generality over special casing

If a pattern fits within the proposal's syntax budget and can be expressed by a single, uniform desugaring rule, support it, even when it is expected to be rare in practice, rather than limiting support to what appears to be the common case.

Part of the motivation is a lesson drawn directly from the two prior attempts at delegation. Both [#1406](https://github.com/rust-lang/rfcs/pull/1406), [#2393](https://github.com/rust-lang/rfcs/pull/2393) restricted delegation in some form leaving multiple possible delegation patterns as future extensions and in both cases the forward-compatibility concerns were never addressed. Therefore in this proposal we want to explore the design space more thoroughly.

This is a default, not an absolute, it may be violated when there is a sufficiently strong reason to do so.

### Design decisions outlined in this RFC

#### Why can delegation items be declared in any position?

Delegation is fundamentally the forwarding of function calls. A regular function in Rust may be a trait method, a method in a trait implementation, an inherent method, or a free function. We can form different combinations based on the position of a caller and a callee:

<details>

<summary> Example: delegating from a trait implementation to an implementation of the same trait.</summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/alloc/src/string.rs#L3635-L3641)

```rust
pub struct Drain<'a> {
    ...
    iter: Chars<'a>,
}

impl Iterator for Drain<'_> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        self.iter.next()
    }
    ...
}
```

</details>

<details>
<summary> Example: delegating from a trait implementation to an implementation of another trait. </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/core/src/iter/adapters/zip.rs#L74-L84)

```rust
trait ZipImpl<A, B> {
    fn next(&mut self) -> Option<Self::Item>;
    ...
}

impl<A, B> Iterator for Zip<A, B>
where
    A: Iterator,
    B: Iterator,
{
    type Item = (A::Item, B::Item);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        ZipImpl::next(self)
    }
    ...
}
```

</details>

<details>
<summary> Example: delegating from an inherent method to a trait implementation. </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/std/src/collections/hash/set.rs#L149-L151)

```rust
impl<T> HashSet<T, RandomState> {
    pub fn new() -> HashSet<T, RandomState> {
        Default::default()
    }
    ...
}
```

</details>

<details>
<summary> Example: delegating from a free function to inherent method. </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/compiler/rustc_ast_pretty/src/pprust/mod.rs#L99-L101)

```rust
pub fn to_string(f: impl FnOnce(&mut State<'_>)) -> String {
	State::to_string(f)
}
```

</details>

etc.

All these combinations appear in real world code via regular calls and each represents a potential target for the delegation feature. Choosing which combinations to support is a design decision driven by multiple factors: the function call resolution algorithm, the available syntax budget, the frequency of the use case and the extensibility to other cases.

Generality is particularly relevant in light of the existing prior art. The two previous delegation RFCs, [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393), deliberately limited delegation to trait methods. Other proposals like [rfcs2375](https://github.com/rust-lang/rfcs/pull/2375) and [rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591) address other use cases through different language mechanisms.


For the callee resolution to any variant is permitted as established in the name resolution section ([?](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)). For the caller we see no reason to restrict (also see [_guiding principles_](#design-guiding-principles)). Accordingly, this proposal supports every combination, rather than special-casing only the most common ones.

_See the following sections for rational/alternatives_:

- [Why are qualified paths used for call disambiguation?](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)

↩ [Reference-level explanation](#reference-level-explanation)

#### why are qualified paths used for call disambiguation? Part 1: high-level view.

Rust distinguishes between two kinds of function invocation. The first one is [method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html), which have the form `receiver.method(args...)`. They are resolved to associated methods that take a receiver argument. Resolution it that case requires additional analysis by the compiler: the receiver may be automatically dereferenced, borrowed or coerced. If more than one method is applicable the compiler emits an error. The second kind is [fully qualified calls](https://doc.rust-lang.org/reference/expressions/call-expr.html#r-expr.call.desugar) which can be used to resolve such ambiguity.


From the delegation's perspective the alternatives can be categorized as follows:

1. Resolve the callee from the method name alone.

    [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) suggested to use method name only to resolve the callee. This covers the most common scenario: delegating a trait implementation to another implementation of the same trait. However this syntax does not generalize naturally to other caller/callee combinations ([?](#why-can-delegation-items-be-declared-in-any-position)) since it can lead to ambiguities in a similar way to method calls.

2. Resolve the callee from the fully qualified path.

   This approach covers every possible caller/callee combination ([?](#why-can-delegation-items-be-declared-in-any-position)) without ambiguity, but it requires more verbose and explicit syntax.

3. Use keywords as disambiguators.

    One of the suggestion from [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) is to use keywords (`trait`/`impl`/`fn`) e.g. (`reuse trait TraitName { expression }`) to disambiguate callee. However, this approach doesn't generalize well to generic contexts. For example, it cannot distinguish between multiple generic implementations of the same trait. Also see next parts ([?](#why-are-qualified-paths-used-for-call-disambiguation-part-2-Self-type)).


The second option has been chosen for this proposal:

1. The first reason is that fully qualified paths already provide a uniform and well‑understood mechanism for disambiguation. Reinventing a separate keyword‑based approach(or any other alternative) would add unnecessary complexity.
2. The second reason is that the first option has already been proposed twice, in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). Rather than attempt the same approach a third time, this proposal comes at the problem from a different angle: name-based resolution can be reintroduced later as pure syntactic sugar layered on top of that mechanism. That keeps the door open to the first option in a forward-compatible way.

_See the following sections for rationale/alternatives_:

- [Why are qualified paths used for call disambiguation? Part 2.](#why-are-qualified-paths-used-for-call-disambiguation-part-2-Self-type)

_See the following sections for future possibilities_:

- [Name-based resolution as sugar](#name-based-resolution-as-sugar)

↩ [Qualified paths and name resolution](#qualified-paths-and-name-resolution)

#### Why are qualified paths used for call disambiguation? Part 2: `Self` type.

We could limit delegation paths to `Trait::name` or `Type::name`, but this is not sufficient to express all delegation patterns. Consider a trait method without a receiver. In regular Rust code calling such a method requires specifying the particular implementation of the trait, for example:

```rust
trait Trait { fn foo(); }

impl Trait for Inner { fn foo() {} }

impl Trait for Outer {
    fn foo() { Trait::foo(); } // ERROR
}

impl Trait for Outer {
    fn foo() { <Inner as Trait>::foo(); } // OK
}
```

The path `Trait::foo` alone is insufficient because multiple implementations may provide the same method. The same principle applies to delegation:

```rust
impl Trait for Outer { reuse Trait::foo; } // ERROR
```

Allowing `Self` in delegation paths makes this possible:

```rust
impl Trait for Outer { reuse <Inner as Trait>::foo; } // OK
```

Therefore, delegation paths should permit `Self` type for the same reason that regular Rust paths use them: they can be necessary to uniquely identify the intended callee (also see [_guiding principles_](#design-guiding-principles)).

_See the following sections for rationale/alternatives_:

- [Why are qualified paths used for call disambiguation? Part 3.](#why-are-qualified-paths-used-for-call-disambiguation-part-3-generic-arguments)

↩ [Qualified paths and name resolution](#qualified-paths-and-name-resolution)

#### Why are qualified paths used for call disambiguation? Part 3: generic arguments.

We could limit delegation paths to `Trait::name`, `Type::name` and `<Type as Trait>::name`, but this is not sufficient to express all delegation patterns. Consider a multiple implementations of the same trait with generic parameters. Rust code calling a method of such trait requires specifying the particular generic arguments, for example:

```rust
trait Trait<T> { fn foo(&self) {} }

impl Trait<i32> for Inner { fn foo(&self) {} }

impl Trait<()> for Inner { fn foo(&self) {} }

impl<T> Trait<T> for Outer {
    fn foo() { Trait::foo(); } // ERROR
}

impl<T> Trait<T> for Outer {
    fn foo(&self) { Trait::<()>::foo(&self.0) } // OK
}
```

The path `Trait::foo` alone is insufficient because multiple implementations may provide the same method. The same principle applies to delegation:

```rust
impl<T> Trait<T> for Outer { reuse Trait::foo { self.0 } } // ERROR
```

Allowing generic arguments in delegation paths makes this possible:

```rust
impl<T> Trait<T> for Outer { reuse Trait::<()>::foo { self.0 } } // OK
```

Therefore, delegation paths should permit generic arguments for the same reason that regular Rust paths use them: they can be necessary to uniquely identify the intended callee (also see [_guiding principles_](#design-guiding-principles)).

TODO: part 4 - type bindings

↩ [Qualified paths and name resolution](#qualified-paths-and-name-resolution)

#### Why is the delegation resolution the trait being implemented in trait implementations?

With _“Refined trait implementations”_ RFC ([rust-lang/rfcs#3245](https://github.com/rust-lang/rfcs/pull/3245))  an implementation signature may be more specific than the one declared in the trait.

If delegation item is in a trait implementation (e.g. `impl Trait for Type { /*delegate foo*/ }`) we have two opportunities:

1. inherit information from the function resolved via callee path

    This option allows to support refined implementations

2. inherit information from the trait method itself

    This option allows to support cases where callee have different signature, but can be called to due to the arguments or return value coercion:

    ```rust
    trait MirPass<'tcx> {
        fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>);
    }

    trait MirLint<'tcx> {
        fn run_lint(&self, tcx: TyCtxt<'tcx>, body: &Body<'tcx>);
    }

    pub(super) struct Lint<T>(pub T);

    impl<'tcx, T> MirPass<'tcx> for Lint<T>
    where
        T: MirLint<'tcx>
    {
        fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
            self.0.run_lint(tcx, body)
        }
    }
    ```
    Here, `run_lint` accepts `&Body<'tcx>`, while the trait method `run_pass` requires `&mut Body<'tcx>`. The delegation can work because `&mut T` can coerce to `&T`. However, if the generated function copied its signature from the callee, `run_pass` would instead take `&Body<'tcx>`, violating the trait definition and resulting in a compilation error.

The `#[refine]` attribute proposed by RFC 3245 could potentially be used for changing the behavior from one to another. We suggest inheriting signatures from the trait by default.

↩ [Desugaring of individual delegation](#desugaring-of-individual-delegation)

#### Why is visibility manually added instead of being copied from the callee?

Delegation item is a distinct item that may deliberately want different behavior than its callee. This also avoids ambiguity for users about whether omitting a visibility modifier makes the delegation item private or causes it to inherit the callee's visibility.

_See the following sections for unresolved questions_:

- [Should the visibility of the delegation item be restricted?](#should-the-visibility-of-the-delegation-item-be-restricted)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why doesn't a delegation item provide syntax for introducing its own generics?

Consider the example:

```rust
pub fn to_vec<T: ConvertVec, A: Allocator>(s: &[T], alloc: A) -> Vec<T, A> {
    T::to_vec(s, alloc)
}
```

In principle, we could support this delegation pattern with syntax such as `reuse<T: ConvertVec, A: Allocator> T::to_vec;`. However, this would exceed our syntax budget(See [_guiding principles_](#design-guiding-principles)).

↩ [Reference-level explanation](#reference-level-explanation)

#### Why doesn't a delegation item provide syntax for arguments or return value transformations?

There are several transformations one might reasonably want from the delegation feature:
- Return value: converting the callee's return type with `.into()`, unwrapping a `Result`/`Option` with `.unwrap()`,  awaiting a future the callee returns with `.await`. e.t.c.
- Input arguments: reordering arguments or calling a method like `.as_ref()`, e.t.c.

To support these transformations in their most general form, delegation items would need something closer to pre-processing and post-processing closures. We do not support these in the RFC, in accordance with our [_guiding principles_](#design-guiding-principles).

↩ [Reference-level explanation](#reference-level-explanation)

#### Why are attributes manually added instead of being copied from the callee?

Attributes may affect diagnostics, linking, documentation, or the item's public API contract. Delegation item is a distinct item that may deliberately want different behavior than its callee. Auto-inheriting attributes would also mean a delegation item's behavior could change silently whenever the callee's attributes change, with no corresponding edit at the delegation site.

_See the following sections for unresolved questions_:

- [Which attributes should be added by default?](#which-attributes-should-be-added-by-default)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why is list delegation supported?

The syntax cost of supporting it is negligible compared with the benefit. Specifically:

1. Individual delegation is very close to a regular function call in terms of the amount of code written and is not particularly useful on its own. One of the main benefits of delegation comes from being able to delegate multiple items at once, avoiding repetitive declarations.
2. It is not a new concept in Rust, as `use` declarations already support lists.
3. Some form of it appears in many prior attempts at delegation, demonstrating users' interest in this capability:
   1. `use expression for name_1, name_i` in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406)
   2. `delegate fn name_1, fn name_i to expression` in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393)
   3. `export path . { sel_1, ..., sel_n }` in [Scala 3](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)


↩ [Reference-level explanation](#reference-level-explanation)

#### Why is glob delegation supported?

The syntax cost of supporting it is negligible compared with the benefit. Specifically:

1. Individual delegation is very close to a regular function call in terms of the amount of code written and is not particularly useful on its own. One of the main benefits of delegation comes from being able to delegate multiple items at once, avoiding repetitive declarations.
2. It is not a new concept in Rust, as `use` declarations already support globs.
3. Some form of it appears in many prior attempts at delegation, demonstrating users' interest in this capability:
   1. `use expression` in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406)
   2. `delegate * to expression` in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393)
   3. `by` clause forwards an entire interface in one declaration in Kotlin.
   4. `#[delegate(Trait)]` delegates every method of `Trait` in [ambassador](https://crates.io/crates/ambassador).
   5. `export name.*` in [Scala 3](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why is renaming supported?

The syntax cost of supporting it is negligible compared with the benefit. Specifically:

1. It is not a new concept in Rust, as `use` declarations already support renaming.
2. Some form of it appears in many prior attempts at delegation, demonstrating users' interest in this capability:
   1. `#[call(name)]` attribute in [delegate](https://crates.io/crates/delegate)
   2. in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) these are possible extensions
   3. `export A as B` in [Scala 3](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why are function qualifiers copied unchanged?

The function header comprises qualifiers such as `const`, `async`, `unsafe`, `extern "ABI"`. The following alternatives exist:

1. Behave identically to regular functions

    Delegation items use the same defaults: `const`, `async`, `unsafe` are omitted, `extern "Rust"` is assigned. Specifying a qualifier overrides the corresponding default for the generated function. This works when the callee also uses the default qualifiers. If they don't:

    - `const`: If the callee is `const` and the delegation item is not, there is no problem: a `const` function can be called from a non-`const` function. However, the delegation item cannot be called from a const context unless `const` is also specified on the delegation item.
    - `ABI`: A mismatch here does not prevent the call from compiling, but it is difficult to see where that would be useful, and the user usually would have to restate ABI for the delegation item.
    - `unsafe`: Calling an `unsafe` function from a non-`unsafe` function requires wrapping the call in an `unsafe` block. We do not want this to happen silently, so the delegation item would have to be marked `unsafe`. Otherwise, the compiler would emit an error.
    - `async`: forwarding to an `async` callee from a non-`async` delegation item isn't possible without changing what gets generated. TODO

2. Inherit qualifiers from the callee.

The proposal chooses to inherit all function qualifiers from the callee unchanged. The main problem with first approach is verbosity. Matching the callee's qualifiers is essentially the only sensible choice, yet that approach would force users to repeat qualifiers for delegation items.

↩ [Desugaring of individual delegation](#desugaring-of-individual-delegation)

#### Why are the delegation resolution's own generic parameters substituted as arguments to the final segment?

Suppose we have a delegation item:

```rust
fn foo<T>(x: i32) {}
reuse foo as bar;
```

Two possible options to generate call are as follows:
- propagate generic parameters to the call:
  ```rust
  fn bar<T>() { foo::<T>() } // Ok
  ```
- do not propagate generic parameters to the call:
  ```rust
  fn bar<T>() { foo() } // ERROR: type annotations needed
  ```

The first option should be chosen because otherwise the generated call may fail with a type inference error.

↩ [Desugaring of individual delegation](#desugaring-of-individual-delegation)

#### Why are statements not passed to the call?

Suppose we have a delegation item:

```rust
reuse path::name { let x = something; self.get(x) }
```

Two possible options to generate call are as follows:
- pass the block expression unchanged:
  ```rust
  path::name(..., { let x = something; self.get(x) }, ...,)
  ```
- hoist the statements out of the block:
  ```rust
  let x = something;
  path::name(..., self.get(x), ...,)
  ```

TODO: the choice (https://github.com/rust-lang/rfcs/pull/3530#issuecomment-2197170600)

↩ [Desugaring of individual delegation](#desugaring-of-individual-delegation)

#### Why is the target expression a block expression?

Unlike [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) a block was chosen over a bare expression (e.g. a hypothetical `reuse prefix::name from expr;`) because a block expression can contain many statements. While having multiple statements during delegation is expected to be a niche use case, anchoring the syntax to the most general form fits is consistent with our [guiding principles](#design-guiding-principles).

↩ [Target expression](#target-expression)

#### Why can the block expression be omitted?

The `;` form is effectively an alias for `{ self }`, providing a more ergonomic way to delegate free functions and methods without a receiver.

↩ [Target expression](#target-expression)

#### Why target expression is not restricted?

In the feedback to the [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) it was suggested that delegation be limited to fields. This suggestion was adopted in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). However we see no compelling reason for this restriction either from an implementation perspective or from the perspective of the language itself. Also see [_guiding principles_](#design-guiding-principles).

↩ [Target expression](#target-expression)

#### Why is delegation of variadic functions not supported?

TODO: find github issue

↩ [Desugaring of individual delegation](#desugaring-of-individual-delegation)

#### Why is generic parameter remapping needed?

We cannot simply copy the signature and where-clauses as-is:

```rust
impl<K, V, A: AllocatorClone> BTreeMap<K, V, A> {
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    { /* impl */ }
}
...
impl<T, A: AllocatorClone> BTreeSet<T, A> {
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.map.contains_key(value)
    }
}
```

Suppose we replace the implementation of  `BTreeSet::contains` with delegation item `reuse BTreeMap::contains { self.map }`. We cannot merely create a syntactically equivalent copy because `K` defined in `BTreeMap` must be remapped to `T` defined in `BTreeSet`.

↩ [Generics remapping](#generics-remapping)

#### What happens if undefined generic parameters remain after substitution?

```rust
impl<K, V, A: AllocatorClone> BTreeMap<K, V, A> {
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    { /* impl */ }
}
...
impl<T, A: AllocatorClone> BTreeSet<T, A> {
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.map.contains_key(value)
    }
}
```

Suppose we replace the implementation of  `BTreeSet::contains` with delegation item `reuse BTreeMap::contains { self.map }`. `K` parameter defined in `BTreeMap` has not been substituted, so there are several options we could consider:

1. Report an error.
2. We can try to infer from the given context:
   1. From the target expression: `typeof(self.map) == BTreeMap::<T, ()>`

      We would need to typecheck the function body before generating the full signature, which is not possible with the current compiler architecture. TODO: same problem as for inherent impls. Add link.

   2. Compiler can use some sort of heuristic (e.g., positional 1:1 matching of parameters defined in the implementation header or substituting parameters with the same names). But this approach is fragile and fails whenever generic parameters are reordered or partially instantiated.

3. TODO: generate. Corner case: fn to trait method.

In this proposal, we suggest using the “report an error” option because it is the most conservative approach and requires generic arguments to be specified explicitly. Once compiler architecture is advanced enough we can implement more sophisticated inference.

↩ [Generics remapping](#generics-remapping)

#### Why are generated generic parameters renamed?

Even if compiler can treat parameters with colliding names as distinct parameters without breaking anything, it is still be better to do renaming for more understandable error messages.

↩ [Generics remapping](#generics-remapping)

### Alternatives to this RFC

TODO: think about https://github.com/BennoLossin/rfcs/blob/field-projection-v2/text/3735-field-projections.md

#### Macros

See [_Prior art_](#prior-art) for a closer look at the two most widely used crates for this, [delegate](https://crates.io/crates/delegate) and [ambassador](https://crates.io/crates/ambassador).

Both show that delegation can already be built as a library, with no change to the language, and both are mature and reasonably ergonomic. However, both are ultimately limited by what a macro can see: macros do not have access to type information such as the callee's resolved signature or the methods of a trait.

Closing this gap fully would require the macro to see type information during expansion, which is the [reflection](#reflection) capability discussed as an alternative below.

#### Reflection

An alternative approach to delegation in Rust would be some form of compile-time reflection. Given the ability to inspect type information such as function signatures during macro expansion, delegation could be implemented as a third-party library, removing the need for dedicated language support.

However, reflection is a large and complex feature that may take years to implement and stabilise. Even if it becomes available it is not clear that it would be the suitable vehicle for delegation.

Work in this direction is already being explored. See [reflection project goal](https://github.com/rust-lang/rust-project-goals/issues/406).

#### Embedding

Rust could instead adopt some form of type embedding (See Go in [Prior art](#prior-art)), where an anonymous field's methods are automatically "promoted" onto the outer struct's method set.

[rust-lang/rfcs#2431](https://github.com/rust-lang/rfcs/issues/2431), opened in 2018, sketches a mechanism for Rust. The issue was posted as a rough idea seeking feedback, but it received little response and remains open with no further activity.

#### Inheritance

Rust could instead adopt some form of inheritance closer to what object-oriented languages provide. However, inheritance has been discussed extensively in the context of Rust, and it is generally not considered aligned with the language's design philosophy.

Also see [Rust book](https://doc.rust-lang.org/book/ch18-01-what-is-oo.html#inheritance-as-a-type-system-and-as-code-sharing) for why.

## Prior art
[prior-art]: #prior-art

- [Delegation or similar mechanisms in other languages](#delegation-or-similar-mechanisms-in-other-languages)
- [Related proposals in Rust](#related-proposals-in-Rust)
- [Crates](#crates)
- TODO: other discussions

### Delegation or similar mechanisms in other languages

#### [Export clauses in Scala](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)

Scala 3's `export` clause has the form `export path . { sel_1, ..., sel_n }` and defines aliases for selected members of an object.

<details>

<summary> Example: Scala export clause.</summary>

```scala
class Inner:
  def hello(): String = "hello"

class Outer(inner: Inner):
  export inner.hello

@main def run(): Unit =
  println(new Outer(new Inner()).hello())
```

</details>

Its selectors line up closely with this proposal's three delegation forms: a single selector corresponds to individual delegation, multiple selectors correspond to list delegation, and a wildcard selector (`*`) corresponds to glob delegation.

`x as y` renames a member on export, the very same `as` keyword this RFC uses for [renaming](#why-is-renaming-supported).

#### [Delegation in Kotlin](https://kotlinlang.org/docs/delegation.html)

Kotlin supports interface delegation natively via a `by` clause on the supertype list: `class Derived(b: Base) : Base by b` implements `Base` for `Derived` by forwarding every one of its methods to `b`. It is close in spirit to this proposal's glob delegation.

Kotlin also lets `Derived` override individual delegated members instead of taking all of them from `b`.

Kotlin [extends](https://kotlinlang.org/docs/delegated-properties.html) the same `by` keyword to individual properties, e.g. `val x: Int by lazy { computeX() }`. There, the expression after `by` is a delegate object providing `getValue` and `setValue` operator functions that the compiler invokes whenever `x` is read or written. This is a related but distinct feature with no direct equivalent proposed here, since Rust has neither properties nor a similar mechanism.

#### [Type embeddings in Go](https://go.dev/ref/spec#Struct_types)

Go has no inheritance either, and addresses the same problem through struct embedding. A struct field declared with only a type, no name, is _embedded_. Embedded value's fields and methods become available directly on the outer struct (`outer.Method()` instead of `outer.inner.Method()`).

<details>

<summary> Example: Go struct embedding.</summary>

```go
package main

import "fmt"

type Inner struct{}

func (Inner) Hello() {
    fmt.Println("hello world")
}

type Outer struct {
    Inner
    Name string
}

func main() {
    o := Outer{Inner: Inner{}, Name: "name"}
    o.Hello() // promoted from Inner; no o.Inner.Hello() needed
}
```

</details>

### Related proposals in Rust

TODO: check https://github.com/GuillaumeGomez/rfcs/blob/derive-deref/text/0000-derive-deref.md

#### [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) (2015, closed)

Delegation was first proposed in a [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406). This RFC introduces a new syntax within trait `impl` blocks, permitting a type to forward an entire trait implementation (or selected items) to a field or arbitrary expression that already implements that trait. The proposed syntax takes the forms:
- `impl Trait for Type { use expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { use expression for name_1 (, name_i)*; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `typeof(expression)` implements `Trait`.

Main reasons for proposal rejection:

1. _Unclear semantics._ It's not clear what kinds of expressions are allowed in the delegation body. Underspecified `self` behavior. The mechanism for desugaring is not defined. Also see [comment](https://github.com/rust-lang/rfcs/pull/1406#issuecomment-269175112).
2. _Forward compatibility._ The RFC intentionally leaves many features for future work, but there was insufficient evidence that the proposed design could be clearly extended to those features without breaking semantics and requiring a redesign.

#### [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) (2018, closed)

Delegation was proposed again in [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). The design was stricter to address the semantics ambiguities of the earlier proposal. The syntax takes the forms:
- `impl Trait for Type { delegate * to expression; }` - delegates all methods of the trait. <br>
- `impl Trait for Type { delegate fn name_1 (, fn name_i)* to expression; }` - delegates a subset of the trait's methods (one or more, listed by name).

where `expression` resolves to a field of `self` (e.g., `self.field`) and `typeof(expression)` implements `Trait`.

Delegation is allowed only for methods that take a receiver by value, by reference or by mutable reference. Proposed desugaring scheme translates delegation item into [method call](https://doc.rust-lang.org/reference/expressions/method-call-expr.html).

Main reasons for proposal rejection:

1. The second proposal was [postponed](https://github.com/rust-lang/rfcs/pull/2393#issuecomment-816822011) due to the lang team bandwidth.
2. Additionally, forward compatibility concerns were never fully addressed.

#### [rfcs2375](https://github.com/rust-lang/rfcs/pull/2375) (2018, open)

This RFC proposes an `#[inherent]` attribute that allows a trait implementation's methods to be called directly on a type without bringing the trait into scope. For example, given:

```rust
#[inherent]
impl Bar for Foo { ... }
```

The methods defined in `Bar` can be called directly on instances of `Foo`, even if `Bar` is not in scope. The RFC defines `#[inherent]` as sugar for a forwarding inherent method:

```rust
impl Foo {
    #[inline]
    pub fn bar(&self) { <Self as Bar>::bar(self); }
}
```


Later on the PR, [nikomatsakis proposed](https://github.com/rust-lang/rfcs/pull/2375#issuecomment-1722647937) replacing `#[inherent]` with `use`. Which is almost the same as `pub reuse Bar::bar;` delegation item under this RFC.

#### [rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591) (2024, merged)

This RFC allows a use declaration to bring a trait's associated functions and constants into scope by path, e.g. `use SomeTrait::some_fn;`. This is not delegation: `use Trait::func` creates a local name for an existing associated function and does not define a new item. However, the same use case can be expressed through the delegation feature.

### Crates

#### [crates.io/delegate](https://crates.io/crates/delegate)

The most used crate for delegation. It implements the `delegate!` declarative macro, which delegates method calls to selected expressions.

<details>

<summary> Example: delegate macro.</summary>

```rust
struct Inner;
impl Inner {
    pub fn method(&self, num: u32) -> u32 { num }
}

struct Wrapper {
    inner: Inner
}

impl Wrapper {
    delegate! {
        to self.inner {
            // calls method_res, unwraps the result, then calls into
            #[unwrap]
            #[into]
            #[call(method_res)]
            pub fn method_res_into(&self, num: u32) -> u64;
        }
    }
}
```

</details>

_Strengths_:

1. It supports a broad range of transformations through attributes such as `#[into(u64)]`, `#[unwrap]`, `#[await(true/false)]` and many others, which can modify the signature or body of the generated method. This makes the macro applicable to a wide range of delegation patterns.
2. It is not limited to trait implementations.

_Weaknesses_:

1. Declarative macros has no access to the callee's actual signature. Every delegated method's signature must be restated by hand in the macro definition.


#### [crates.io/ambassador](http://crates.io/crates/ambassador)

The second most popular crate for delegation. in contrast with [delegate](https://crates.io/crates/delegate), procedural macros are used, not declarative ones.

<details>

<summary> Example: ambassador macro.</summary>

```rust
use ambassador::delegatable_trait;

#[delegatable_trait]
pub trait Trait {
    fn method(&self, input: &str) -> String;
}

pub struct Inner;

impl Trait for Inner {
    fn method(&self, input: &str) -> String {
        // impl
    }
}

#[derive(Delegate)]
#[delegate(Trait)]
pub struct Outer(Inner);
```

</details>

_Strengths_:

1. Unlike delegate, the callee's signature does not need to be restated at each delegation site.

_Weaknesses_:

1. It supports a narrower range of delegation patterns then delegate: Ambassador can only delegate trait implementations, delegates only to fields, and does not support transformations of the delegated method's signature.
2. The trait being delegated must be annotated with `#[delegatable_trait]`. For foreign traits, it must instead be re-declared locally with #`[delegatable_trait_remote]`.
3. In `#[delegate(..., target = "self")]` or `#[delegate(..., where = "A: Shout")]` expressions are specified as strings rather than using regular Rust syntax.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

The questions below are not expected to block acceptance of this RFC. Each is either a minor detail that can be settled during implementation or before stabilization.

### Which attributes should be added by default?

Certain attributes may be reasonable to add or inherit from the callee by default. The current implementation adds the `#[inline]` attribute.

There should also be a way to opt out of default attributes when they are not desired. For `#[inline]`, this may be done with `#[inline(never)]` on the delegation item, but the appropriate mechanism depends on the attribute, and some attributes may have no corresponding way to opt out.

↩ [Why are attributes manually added instead of being copied from the callee?](#why-are-attributes-manually-added-instead-of-being-copied-from-the-callee)

### Should the visibility of the delegation item be restricted?

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

We prefer to leave all control to the user while also adding a lint that prevents a generated function from having greater visibility than the callee.

↩ [Why is visibility manually added instead of being copied from the callee?](#why-is-visibility-manually-added-instead-of-being-copied-from-the-callee)

### What keyword should be used?

The draft uses `reuse`, but other options like `delegate` or `forward` could be considered.

↩ [Reference-level explanation](#reference-level-explanation)

## Future possibilities
[future-possibilities]: #future-possibilities

Several extensions could be added on top of the core feature without changing its fundamental semantics. At the same time, the scope for such extensions is relatively limited, and they would primarily provide syntactic conveniences rather than introduce fundamentally new functionality.

### Name-based resolution as sugar

A shorter syntax that infers the callee from a bare method name could be layered on top of fully qualified paths.

↩ [Why are qualified paths used for call disambiguation](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)

### Support delegating types and consts

We could support desugaring for types and consts as follows:

```rust
impl Trait for S {
    reuse Trait::{Item, MAX, func} { self.0 }
}

impl Trait for S {
    type Item = <F as Trait>::Item;
    const MAX = <F as Trait>::MAX;
    fn func(&self) -> u32 {
        <F as Trait>::func(&self.0)
    }
}
```

However, there are 2 complexities:
1. Types live in the type namespace, while functions and constants live in the value namespace. A single qualified path doesn't say which namespace to pull from, so `Trait::name` is ambiguous whenever `Trait` has both an associated type and an associated fn/const called `name`.

   This can be solved by introducing a disambiguator for types. One of the suggestions from [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) is to use `fn`/`type`/`const` keywords.

2. TODO: impl


Based on these notes we would like to postpone delegation of types and constants.

↩ [Reference-level explanation](#reference-level-explanation)
