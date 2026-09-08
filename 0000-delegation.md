
- Feature Name: (fill me in with a unique ident, `fn_delegation`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Provide a syntactic sugar to automatically forward function calls.

## Terminology

The following terminology is frequently used in this proposal:

- _delegation item_ - a new item kind introduced by this proposal, declared with the `reuse` keyword, that generates a function or method which forwards its arguments to a specified callee.
- _target expression_ - an optional block expression that transforms the delegation item's first argument before that argument is forwarded to the resolved callee.
- _renaming_ - the ability to give the generated function a name that differs from the callee's name.
- _parent context_ - the parent item in which the delegation item appears. This can be a module (for free functions), a trait implementation, a type implementation or a trait(for associated items).
- _desugaring_ - the translation from a delegation item into regular function calls.
- _delegation pattern_ - TODO

## Conventions

This RFC draws on the experimental implementation tracked in [rust-lang/rust#118212](https://github.com/rust-lang/rust/issues/118212).


TODO: links to rational/external/other sections </br>
TODO: notes to implementation experience, other notes </br>
TODO: examples </br>
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

The is simply forwards a method call to a field. In practice the required repetition may even discourage the use of newtypes despite their advantages for type safety and abstraction. This situation highlights a gap in Rust’s ergonomics. While Rust provides powerful mechanisms for defining abstractions through traits and generics it offers comparatively little support for reusing existing behavior.

This limitation has long been recognized by the Rust community: it has motivated two prior RFCs ([#1406](https://github.com/rust-lang/rfcs/pull/1406), [#2393](https://github.com/rust-lang/rfcs/pull/2393)), a multiple discussions, and several macro crates ([delegate](https://crates.io/crates/delegate) and [ambassador](https://crates.io/crates/ambassador) are most popular amongst them). See [Prior art](#prior-art) for a full discussion of these efforts.

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

The method this adds to `BTreeSet<T>` is called `contains`, not `contains_key` without changing anything else about the delegation.

### Delegating several methods at once

Listing out `is_empty`, `clear` and `len` as three separate reuse items is still three lines whose only real difference is the method name. List delegation collapses them into one:

```rust
impl<T> BTreeSet<T> {
    reuse BTreeMap<T, ()>::{len, is_empty, clear, capacity} { self.map }
}
```

Each generated method gets the receiver its callee needs, not a receiver you have to spell out yourself: `clear` needs to mutate the map, so the method this generates takes `&mut self`, while `len` and `is_empty` only need to read it, so those take the shared reference `&self`. The target expression `{ self.map }` is the same in all four cases.

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

Delegation items can be declared in any position where items are allowed. They are also associated items and may therefore appear in traits and implementations ([?](#why-can-delegation-items-be-declared-in-any-position)). Like other items, delegation items may be annotated with a visibility modifier ([?](#why-is-visibility-manually-added-instead-of-being-inherited-from-the-callee)) and may have attributes applied to them ([?](#why-are-attributes-manually-added-instead-of-being-inherited-from-the-callee)).

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

A delegation item starts with the `reuse` keyword and consists of a fully qualified path, followed by an optional block expression. It comes in three flavors, matching the three forms of `DelegationPath`: individual delegation, list delegation ([?](#why-is-list-delegation-supported)) and glob delegation ([?](#why-is-glob-delegation-supported)). The optional `as IDENTIFIER` allows to expose the delegated function under a different name ([?](#why-is-renaming-supported)). Delegation of types and constants is not supported ([?](#why-is-delegation-of-types-and-constants-not-supported)).

_See the following sections for rationale/alternatives_:

- [Why can delegation items be declared in any position?](#why-can-delegation-items-be-declared-in-any-position)
- [Why is visibility manually added instead of being inherited from the callee?](#why-is-visibility-manually-added-instead-of-being-inherited-from-the-callee)
- [Why are attributes manually added instead of being inherited from the callee?](#why-are-attributes-manually-added-instead-of-being-inherited-from-the-callee)
- [Why is list delegation supported?](#why-is-list-delegation-supported)
- [Why is glob delegation supported?](#why-is-glob-delegation-supported)
- [Why is renaming supported?](#why-is-renaming-supported)
- [Why is delegation of types and constants not supported?](#why-is-delegation-of-types-and-constants-not-supported)

_See the following sections for unresolved questions_:

- [Should the visibility of the delegation item be restricted?](#should-the-visibility-of-the-delegation-item-be-restricted)
- [Which attributes should be added by default?](#which-attributes-should-be-added-by-default)
- [What keyword should be used?](#what-keyword-should-be-used)

_See the following sections for future possibilities_:

- [Support delegating types and consts as part of glob delegation](#support-delegating-types-and-consts-as-part-of-glob-delegation)

### Qualified paths and name resolution

Qualified paths provide an unambiguous way to identify callable items, including trait methods, trait implementation methods, inherent methods, and free functions ([?](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)).

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

TODO: say from whom signature is inherited. For trait impl ... For others ...

_See the following sections for rationale/alternatives_:

- [Why are qualified paths used for call disambiguation](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)

### Target expression

The target expression is an optional [block expression](https://doc.rust-lang.org/beta/reference/expressions/block-expr.html) ([?](#why-is-the-target-expression-a-block-expression)) that transforms the delegation item's first argument before that argument is forwarded to the resolved callee.

Inside that block, `self` refers to TODO

When no block is given the first argument is passed through unchanged. ([?](#why-is-the-block-expression-optional-in-the-target-expression)).

TODO: arbitrary expression, not a field

_See the following sections for rational/alternatives:_

- [Why is the target expression a block expression?](#why-is-the-target-expression-a-block-expression)
- [Why is the block expression optional in the target expression?](#why-is-the-block-expression-optional-in-the-target-expression)

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

- [Design decisions outlined in this RFC](#design-decisions-outlined-in-this-rfc)
- [Alternatives to this RFC](#alternatives-to-this-rfc)
- TODO: we have some statistics and we can add it here

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

TODO: link to 2 rfcs in prior art.

All these combinations appear in real world code via regular calls and each represents a potential target for the delegation feature. Choosing which combinations to support is a design decision driven by multiple factors: the function call resolution algorithm, the available syntax budget, the frequency of the use case and the extensibility to other cases.

For the callee resolution to any variant is permitted as established in the name resolution section ([?](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)). For the caller we see no reason to restrict it as long as it fits within the general desugaring scheme and is likely to be encountered in practice. Accordingly, this proposal supports every combination, rather than special-casing only the most common ones.

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
2. The second reason is that the first option has already been proposed twice, in [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393). Rather than attempt the same approach a third time, this proposal comes at the problem from a different angle: because every callee is already reachable through a fully qualified path, name-based resolution can be reintroduced later as pure syntactic sugar layered on top of that mechanism. That keeps the door open to the first option in a forward-compatible way.

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

Therefore, delegation paths should permit `Self` type for the same reason that regular Rust paths use them: they can be necessary to uniquely identify the intended callee.

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

Together, these cases motivate a general guiding principle: to support the full range of existing Rust path syntax that is useful for identifying a callee, rather than introducing special-case restrictions for delegation. As long as the syntax remains within the proposal's syntax budget, there is no reason to impose a narrower subset.

TODO: part 4 - type bindings

↩ [Qualified paths and name resolution](#qualified-paths-and-name-resolution)

#### Why is visibility manually added instead of being inherited from the callee?

Delegation item is a distinct item that may deliberately want different behavior than its callee. This also avoids ambiguity for users about whether omitting a visibility modifier makes the delegation item private or causes it to inherit the callee's visibility.

_See the following sections for unresolved questions_:

- [Should the visibility of the delegation item be restricted?](#should-the-visibility-of-the-delegation-item-be-restricted)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why are attributes manually added instead of being inherited from the callee?

Attributes may affect diagnostics, linking, documentation, or the item's public API contract. Delegation item is a distinct item that may deliberately want different behavior than its callee. Auto-inheriting attributes would also mean a delegation item's behavior could change silently whenever the callee's attributes change, with no corresponding edit at the delegation site.

<details>

<summary> Example: discrepancy between caller and callee attributes </summary>

[example link](https://github.com/rust-lang/rust/blob/752b9bf8798c2ffc1d3fe2b804c04454366fc6d6/library/alloc/src/io/util.rs#L390-L393)

```rust
#[inline(never)]
fn uninlined_slow_read_byte<R: Read>(reader: &mut R) -> Option<Result<u8>> {
	inlined_slow_read_byte(reader)
}

#[inline]
fn inlined_slow_read_byte<R: Read>(reader: &mut R) -> Option<Result<u8>> {
    ...
}
```

</details>

_See the following sections for unresolved questions_:

- [Which attributes should be added by default?](#which-attributes-should-be-added-by-default)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why is list delegation supported?

The syntax cost of supporting it is negligible compared with the benefit. Some form of it appears in essentially every prior attempt at delegation, demonstrating that users need this capability. It is also not a new concept in Rust, as `use` declarations already support lists.

TODO: links

↩ [Reference-level explanation](#reference-level-explanation)

#### Why is glob delegation supported?

The syntax cost of supporting it is negligible compared with the benefit. Some form of it appears in essentially every prior attempt at delegation, demonstrating that users need this capability. It is also not a new concept in Rust, as `use` declarations already support globs.

TODO: links

↩ [Reference-level explanation](#reference-level-explanation)

#### Why is renaming supported?

The syntax cost of supporting it is negligible compared with the benefit. Some form of it appears in essentially every prior attempt at delegation, demonstrating that users need this capability. It is also not a new concept in Rust, as `use` declarations already support renaming.

TODO: links

↩ [Reference-level explanation](#reference-level-explanation)

#### Why is delegation of types and constants not supported?

Types live in the type namespace, while functions and constants live in the value namespace. A single qualified path doesn't say which namespace to pull from, so `Trait::name` is ambiguous whenever `Trait` has both an associated type and an associated fn/const called `name`.

TODO: what is target expression mean here?
TODO: why constants?

_See the following sections for future possibilities_:

- [Support delegating types and consts as part of glob delegation](#support-delegating-types-and-consts-as-part-of-glob-delegation)

↩ [Reference-level explanation](#reference-level-explanation)

#### Why are function qualifiers inherited unchanged from the callee?


The function header comprises qualifiers such as `const`, `async`, `unsafe`, `extern "ABI"`.

- If the callee is a const function, the generated function is also `const`. This is necessary for the delegation to be usable in const contexts.
- If the callee is `async`, the generated function is also `async`. This is necessary for the delegation to be usable in async contexts.
- If the callee is `unsafe`, the generated function is also `unsafe`. Delegation merely forwards the call and cannot verify the safety contract required by the callee. Therefore, the same safety obligations must be imposed on caller.
- The generated function inherits the same ABI. It would be counterintuitive otherwise.

One further consequence worth noting: because a delegation item's ABI, `unsafe`-ness, and `async`-ness are always identical to the callee's, a delegation item can be coerced to a function pointer or passed anywhere the callee itself could be.

TODO: why don't use callee's as default and override? <br>
TODO: attributes and vis are specified manually while these are inherited. Why?

Programmer who wants a different behavior can still write a wrapper by hand.

↩ [Individual delegation](#individual-delegation)

#### Why is the target expression a block expression?

Unlike [rfcs#1406](https://github.com/rust-lang/rfcs/pull/1406) and [rfcs#2393](https://github.com/rust-lang/rfcs/pull/2393) a block was chosen over a bare expression (e.g. a hypothetical `reuse prefix::name from expr;`) for a 2 reasons:

- A block expression can contain arbitrary statements. While having multiple statements during delegation is expected to be a niche use case, anchoring the syntax to the most general form ensures forward compatibility.
- The language consistently uses block expressions such as `unsafe { ... }`, `async { ... }`, or `gen { ... }` and does not usually place bare expressions outside of function bodies. So this might be better from an ergonomic perspective.

↩ [Target expression](#target-expression)

#### Why is the block expression optional in the target expression?

TODO: The `;` form is effectively an alias for `{ self }`.

↩ [Target expression](#target-expression)

#### Why is delegation of variadic functions not supported?

TODO: find github issue

↩ [Individual delegation](#individual-delegation)

### Alternatives to this RFC

TODO: think about https://github.com/BennoLossin/rfcs/blob/field-projection-v2/text/3735-field-projections.md

#### Macros

See [Prior art](#prior-art) for a closer look at the two most widely used crates for this, [delegate](https://crates.io/crates/delegate) and [ambassador](https://crates.io/crates/ambassador).

Both show that delegation can already be built as a library, with no change to the language, and both are mature and reasonably ergonomic. However, both are ultimately limited by what a macro can see: macros do not have access to type information such as the callee's resolved signature or the methods of a trait.

Closing this gap fully would require the macro to see type information during expansion, which is the [reflection](#reflection) capability discussed as an alternative below.

#### Embedding

TODO: https://github.com/rust-lang/rfcs/issues/2431 + link to Go

#### Inheritance

Rust could instead adopt some form of inheritance closer to what object-oriented languages provide. However, inheritance has been discussed extensively in the context of Rust, and it is generally not considered aligned with the language's design philosophy.

TODO: add links

#### Reflection

An alternative approach to delegation in Rust would be some form of compile-time reflection. Given the ability to inspect type information such as function signatures during macro expansion, delegation could be implemented as a third-party library, removing the need for dedicated language support.

However, reflection is a large and complex feature that may take years to implement and stabilise. Even if it becomes available it is not clear that it would be the suitable vehicle for delegation. TODO: somehow to to disambiguation problem

Work in this direction is already being explored. See [reflection project goal](https://github.com/rust-lang/rust-project-goals/issues/406).

## Prior art
[prior-art]: #prior-art

TODO: other langs <br>
TODO: derive in Haskell?

### Kotlin

Kotlin supports interface delegation natively via a `by` clause on the supertype list: `class Derived(b: Base) : Base by b` implements `Base` for `Derived` by forwarding every one of its methods to `b`. It is close in spirit to this proposal's glob delegation.

TODO: links

### Go lang

Go has no inheritance either, and addresses the same problem through struct embedding. A struct field declared with only a type, no name, is _embedded_.

TODO: continue

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
- TODO

__Weaknesses__:

- Declarative macros like has no access to the callee's actual signature. Every delegated method's signature must be restated by hand in the macro definition.
- TODO


### [crates.io/ambassador](http://crates.io/crates/ambassador)

The second most popular crate for delegation. in contrast with [delegate](https://crates.io/crates/delegate), procedural macros are used, not declarative ones.

__Strengths__:

TODO:

__Weaknesses__:

TODO:

### [rfcs2375](https://github.com/rust-lang/rfcs/pull/2375) (2018)

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

### [rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591) (2024, merged)

This RFC allows a use declaration to bring a trait's associated functions and constants into scope by path, e.g. `use SomeTrait::some_fn;`. This is not delegation: `use Trait::func` creates a local name for an existing associated function and does not define a new item. However, the same use case can be expressed through the delegation feature.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

The questions below are not expected to block acceptance of this RFC. Each is either a minor detail that can be settled during implementation or before stabilization.

### Which attributes should be added by default?

Certain attributes may be reasonable to add or inherit from the callee by default. The current implementation adds the `#[inline]` attribute: inlining is purely an optimisation, so it keeps a  forwarding wrapper as close to zero-cost abstraction as writing the call by hand.

There should also be a way to opt out of default attributes when they are not desired.

↩ [Why are attributes manually added instead of being inherited from the callee?](#why-are-attributes-manually-added-instead-of-being-inherited-from-the-callee)

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

We prefer to leave all control to the user while also adding a deny-by-default lint that prevents a generated function from having greater visibility than the callee.

↩ [Why is visibility manually added instead of being inherited from the callee?](#why-is-visibility-manually-added-instead-of-being-inherited-from-the-callee)

### What keyword should be used?

The draft uses `reuse`, but other options like `delegate` or `forward` could be considered.

↩ [Reference-level explanation](#reference-level-explanation)

## Future possibilities
[future-possibilities]: #future-possibilities

Several extensions could be added on top of the core feature without changing its fundamental semantics. At the same time, the scope for such extensions is relatively limited, and they would primarily provide syntactic conveniences rather than introduce fundamentally new functionality.

### Name-based resolution as sugar

A shorter syntax that infers the callee from a bare method name could be layered on top of fully qualified paths.

↩ [Why are qualified paths used for call disambiguation](#why-are-qualified-paths-used-for-call-disambiguation-part-1-high-level-view)

### Support delegating types and consts as part of glob delegation

Glob delegation could still support delegation of types and constants because it does not specify individual names.

↩ [Why is delegation of types and constants not supported?](#why-is-delegation-of-types-and-constants-not-supported)
