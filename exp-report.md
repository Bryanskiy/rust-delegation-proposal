# Current implementation details

## Finding signature function

There are four entities that are involved in delegation generation:
- Signature function,
- Signature function parent,
- Delegation parent,
- Call path resolution.

```rust
// Signature function parent.
trait Trait<'a, T> {
    // Signature function (and call path resolution).
    fn foo<U, const N: usize>(&self);
}

struct X;
// Delegation parent.
impl X {
    // Delegation.
    reuse Trait::foo;
}
```

The first thing to do is given delegation path `Trait::foo` find its signature function. There are two cases in this procedure:
- Delegation from trait impl
  When delegating from trait impl we should always generate a function whose signature matches the signature of trait, otherwise it would an invalid code:

  ```rust
  trait Trait {
    fn foo();
    fn bar(x: usize)
  }

  mod to_reuse {
    pub fn foo() {}
    pub fn bar(x: String) {}
  }

  struct X;
  impl Trait for X {
    // Those delegations resolve to `Trait::foo` and `Trait::bar`,
    // not to the `to_reuse::foo` and `to_reuse::bar`.
    reuse to_reuse::{foo, bar};

    // Desugaring:
    fn foo() { to_reuse::foo() }
    // Note that we generated function with signature from `Trait`
    // (`x` is of type `usize`), however `bar` accepts `String` so we would
    // get a mismatch types error.
    fn bar(x: usize) { to_reuse::bar(x) }
  }
  ```

- All other cases
  The resolution of the signature function in all other cases matches the resolution of delegation path segment.

## How target expression is generated

## Signature and clauses inheritance, generics

When generating delegation we need to generate generics, inherit clauses and signature. Generics are inherited during AST -> HIR lowering, while clauses and signature are created during HIR analysis. 

### Free-to-free

### Free-to-trait

### Free-to-impl

### Trait-to-free

### Trait-to-trait

### Trait-to-impl

### TraitImpl-to-free

### TraitImpl-to-trait

### TraitImpl-to-impl

### Impl-to-free

### Impl-to-trait

### Impl-to-impl

## Recursive delegations

## Glob and list delegations, deletion of the target expression

When delegating to trait or inherent impl, list or glob delegation can be used:
```rust
trait Trait {
    fn static_f();
    fn by_value(self);
    fn by_ref(&self);
    fn by_mut_ref(&mut self);
}

struct X<T>(T);
impl<T: Trait> X<T> {
    // List delegation.
    reuse Trait::{static_f, by_value} { self.0 }
}

struct X2<T>(T);
impl<T: Trait> X<T> {
    // Glob delegation.
    reuse Trait::* { self.0 }
}
```

List delegation can be used when delegating to traits or inherent impls, glob delegation can be used only when delegating to traits.

Note that one of reused functions has no receiver, thus it would be incorrect to apply target expression in such delegation, that is why we need to remove it. Glob and list delegations are expanded before AST -> HIR lowering, meaning we have standalone delegations for each list or glob delegation. That in turn means that we already allocated `LocalDefId`s for contents of target
expression that was duplicated for each standalone delegation. If we want to remove it for some delegations we need to delete arbitrary user code that is inside delegation's target expression, meaning potential deletion of already allocated inner `LocalDefId`s. Unfortunately, now it is hard to remove already allocated `LocalDefId` with zero performance cost as a lot of data structures that are used inside compiler are optimized for continuous ranges of number-like structs. Moreover, even if we alter existing data structures so they support deletion (or marking ids that should be considered deleted) next we have to think how to prevent leak of deleted ids, as in other features of rust compiler someone may store deleted id somewhere, bypassing our checks, and then access it when it is considered deleted by altered data structures. This require complex analysis of all data structures that are used in the compiler, when they are accessed and how to modify them in performance-friendly manner with an expressive infrastructure that will prevent leak of deleted ids.


## Adjustments for the "receiver" argument

```rust
trait Trait {
    fn by_value(self);
    fn by_ref(&self);
    fn by_mut_ref(&mut self);
}

struct X<T>(T);
impl<T: Trait> X<T> {
    reuse Trait::* { self.0 }
}
```

Consider the example above, when writing `self.0` target expression we implicitly think that it will compile with all three reused functions, however they all have different receivers: by value, by readonly reference and by mutable references. However, we want the code above to compile, adding implicit adjustments to the target expression like in a method call scenario: `self.by_mut_ref()` will automatically cast `self` to `&mut Self` when possible. This strategy was used in delegation for a long time, we simply generated method call and all adjustments that are supported by method call processing routine were automatically applied for delegations. However, there is one problem that made us to rethink this approach: propagation of parent generics. Consider the modified code below:

```rust
trait Trait<U> {
    fn by_value(self);
    fn by_ref(&self);
    fn by_mut_ref(&mut self);
}

struct X<T>(T);
impl<T: Trait<()>> X<T> {
    reuse Trait::* { self.0 }

    // Desugaring:
    fn by_value<U>(self) {
        <Self as Trait<U>>::by_value(self.0)
    }
}
```

In this case we want to generate generic parameter `U` in generated delegations and propagate it into the call path, as it is shown in the snippet above. However, if we generate method call it would be impossible to propagate parent generics, that is why we decided to step away from the method call generation approach and generate usual call and apply adjustments as if we generated method call. That was implemented: we now reuse method call resolution routine in order to find adjustments and apply them to the first argument of the generated call. Thus we can simultaneously propagate parent generics and use implicit adjustments for user's convenience.

Considering adjustments it becomes important how we generate target expression. If we simply generate target expression in the same way as user written it:
```rust
reuse Trait::foo {
    println!("Hello");
    self.0
}

// Desugaring:
fn foo<Self>() {
    <Self as Trait>::foo({
        println!("Hello");
        self.0
    })
}
```

then we would fail to apply adjustments, as by rust design block returns by-value. Instead we generate target expression in the following form:
```rust
// Desugaring:
fn foo<Self>() {
    println!("Hello");
    <Self as Trait>::foo(self.0)
}
```

We require that target expression has it final-return expression and all statements before it are put outside of the call path, thus we extend the number of cases we can apply adjustments to.

## `Self`-type mapping

## One-line trait reuse

```rust
trait T {
  fn foo(&self);
}

struct S;
impl T for S { ... }

struct Wrapper(S);
reuse impl T for Wrapper { self.0 }
```

The core idea is that we already have support for glob reuse, so in this scenario we want to transform one-line reuse into a trait impl block with a glob reuse in the following way:

```rust
// Before:
reuse impl T for Wrapper { self.0 }

// After:
impl T for Wrapper {
  reuse T::* { self.0 }
}
```

Other syntax possibility is trying to shorten one-line reuse by replacing `impl` keyword with `reuse` keyword:
```rust
reuse T for Wrapper { self.0 }
```

In this case implementation may become more complicated, and the syntax more confusing, as keywords such as `const` or `unsafe` will precede `reuse`, and there are also generics:
```rust
unsafe reuse<T1, T2> T for Wrapper { self.0 }
```

In the first (currently implemented) version reuse is placed in the beginning of the item, and it is clear that we will reuse trait implementation, while in the second, shorter version, the `reuse` keyword may be lost in generics and keywords that may precede `impl`.

## Inherited and generated attributes

When generating delegation we inherit and generate attributes. More specifically we inherit inline attribute if it is specified on the signature function. There are some attributes which are generated: `#[inline(Hint)]` and `#[must_use("reason")]`. They are generated only if there is no same attribute present of the signature function. In case of the recursive delegations, the attribute is propagated on the whole chain of recursive delegations.

# Unresolved questions

## Resolution of type-relative path during AST -> HIR lowering