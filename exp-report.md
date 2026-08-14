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
    // Signature function (and call path resolution in this case).
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

User can either specify generic arguments in delegation or use single infer (`'_` for lifetimes or `_` for types and consts) to indicate that this generic param should be generated.

### Free-to-free

In case of free-to-free delegation we simply copy generics from signature function:

```rust
fn foo<'a, 'b: 'b, T, const X: usize>(&'a [T; X]) {}

reuse foo as bar;
reuse foo::<'static, (), 123> as baz;
reuse foo::<'_, (), _> as oof;

// Desugaring:
#[attr = Inline(Hint)]
fn bar<'b, T, const X: _>(arg0: _) -> _ where 'b:'b { foo::<'b, T, X>(arg0) }

#[attr = Inline(Hint)]
fn baz(arg0: _) -> _ { foo::<'static, (), 123>(arg0) }

#[attr = Inline(Hint)]
fn oof<'b, const X: _>(arg0: _) -> _ where 'b:'b { foo::<'b, (), X>(arg0) }
```

There are several moments to notice:
- We generated hacky predicate `'b: 'b` in the bar delegation, as we can not generate standalone lifetime `'b` which will not be considered early-bound, this predicate is not used anywhere except to pass this lifetime forward,
- In the second `baz` delegation user explicitly specified generic args, so we did not generated any generic params,
- In the third example user used two infers at first and third generic argument positions, so we generate and propagate generic params for them, leaving the second generic argument untouched.

### Free-to-trait

One of the most interesting cases, as traits contain implicit `Self` generic parameter, that needs to be considered when generating delegation:

```rust
trait Trait<'a, T, const B: bool> {
    fn foo<'e, 'b: 'b, U, const X: usize>(arr: &'a [T; X]) {
    }
}

struct S;
impl<'a, T, const B: bool> Trait<'a, T, B> for S {}

reuse Trait::foo as bar;
reuse Trait::<'static, (), false>::foo as bar1;
reuse Trait::<'static, (), false>::foo::<'static, (), 123> as bar2;
reuse <S as Trait>::foo as bar3;

// Desugaring:
#[attr = Inline(Hint)]
fn bar<'a, 'b, Self, T, const B: _, U, const X: _>(arg0: _) -> _ where 'a:'a,
    'b:'b { <Self as Trait::<'a, T, B>>::foo::<'b, U, X>(arg0) }

#[attr = Inline(Hint)]
fn bar1<'b, Self, U, const X: _>(arg0: _) -> _ where
    'b:'b { <Self as Trait::<'static, (), false>>::foo::<'b, U, X>(arg0) }

#[attr = Inline(Hint)]
fn bar2<Self>(arg0: _)
    ->
        _ {
    <Self as Trait::<'static, (), false>>::foo::<'static, (), 123>(arg0)
}

#[attr = Inline(Hint)]
fn bar3<'a, 'b, T, const B: _, U, const X: _>(arg0: _) -> _ where 'a:'a,
    'b:'b { <S as Trait::<'a, T, B>>::foo::<'b, U, X>(arg0) }
```

User can specify generic arguments either in parent, child or parent and child segments. Moreover user can specify explicit `Self` parameter: `<S as Trait>::foo`. We always generate call path of the following pattern: `<Self as Trait</* propagated generic params if any */>>::foo::</* propagated generic params if any */>(...)`, where `Self`, parent and child generics can be either params of user-specified arguments.

The pattern of generated generic params is as follows: for `reuse Trait::foo` we have `[parent lifetimes], [child lifetime], Self, [parent types can consts], [child types and consts]`.

### Free-to-impl



### Trait-to-free

In trait to free delegation everything is relatively simple:

```rust
fn foo<'a: 'a, T, const X: usize>() {}

trait Trait {
    reuse foo;
    reuse foo::<'static, (), 123> as bar;

    // Desugaring:
    #[attr = Inline(Hint)]
    fn foo<'a, T, const X: _>() -> _ where 'a:'a { foo::<'a, T, X>() }
    #[attr = Inline(Hint)]
    fn bar() -> _ { foo::<'static, (), 123>() }
}
```

Note that when delegating from trait to free function function can not become a method, so if we have:
```rust
fn foo(x: impl Trait) {}

trait Trait {
    reuse foo;

    // Desugaring:
    #[attr = Inline(Hint)]
    fn foo<impl Trait>(arg0: _) -> _ { foo(arg0) }
}
```

`foo` was not generated with `self` receiver instead it is treated as a "static" function with a single argument of type `impl Trait`.


### Trait-to-trait

In trait to trait delegation we follow approximately the same rules as during trait to free delegation, adding parent segment generics:
```rust
trait Trait<'a, T, const B: bool> {
    fn static_f<'b, U, const X: usize>() {}
    fn foo(self) {}
}

trait Trait2 {
    reuse Trait::static_f as f0;
    reuse Trait::<'static, (), true>::static_f as f1;
    reuse Trait::<'static, (), true>::static_f::<'static, (), 123> as f2;
    
    reuse Trait::foo as f3;
    reuse Trait::<'static, (), false>::foo as f4;

    // Desugaring:
    #[attr = Inline(Hint)]
    fn f0<'a, T, const B: _, U, const X: _>() -> _ where
        'a:'a { Trait::<'a, T, B>::static_f::<U, X>() }

    #[attr = Inline(Hint)]
    fn f1<U, const X: _>()
        -> _ { Trait::<'static, (), true>::static_f::<U, X>() }

    #[attr = Inline(Hint)]
    fn f2()
        -> _ { Trait::<'static, (), true>::static_f::<'static, (), 123>() }

    #[attr = Inline(Hint)]
    fn f3<'a, T, const B: _>(self: _) -> _ where
        'a:'a { Trait::<'a, T, B>::foo(self) }

    #[attr = Inline(Hint)]
    fn f4(self: _) -> _ { Trait::<'static, (), false>::foo(self) }
}
```

We do not generate explicit `Self` generic parameter as in free to trait delegation. Note that `Self` implicit generic parameter corresponds to `Trait2`.

### Trait-to-impl

### TraitImpl-to-free

In all trait impls delegations the signature function always resolves to a trait function, so the resolution of a call path and signature may differ. Moreover, as we need to generate a delegation that will match it signature in trait, meaning that no matter whether generic arguments are specified in the call path we should generate generic params from the signature function. Here we may encounter situations where generic params of the function and generic params of the call path function may differ. So in this case we just propagate generic params of delegation into the call path (if user did not provide generic arguments), and if they do not match then we will get an error (if generic arguments of the child are not specified and they will not be propagated we will get an error anyway).

```rust
fn foo<T, U>(x: usize) {}

trait Trait {
    fn bar();
    fn bar1(self);
    fn bar2<T, U>(x: usize);
    fn bar3<T, U, V>(x: usize);
}

struct X;
impl Trait for X {
    reuse foo::<(), ()> as bar;
    reuse foo as bar1;
    reuse foo as bar2;
    reuse foo as bar3;

    #[attr = Inline(Hint)]
    fn bar() -> _ { foo::<(), ()>() }

    #[attr = Inline(Hint)]
    fn bar1(self: _) -> _ { foo(self) }

    #[attr = Inline(Hint)]
    fn bar2<T, U>(arg0: _) -> _ { foo::<T, U>(arg0) }

    // Generics do not match but we still propagate them.
    #[attr = Inline(Hint)]
    fn bar3<T, U, V>(arg0: _) -> _ { foo::<T, U, V>(arg0) }
}
```

### TraitImpl-to-trait

Same as trait impl but we ignore parent generics of the delegation call path:

```rust
trait FromTrait<'a, B, C> {
    fn foo<T, U>(&self, x: usize) {}
}

trait Trait {
    fn bar();
    fn bar1(self);
    fn bar2<T, U>(&self, x: usize);
    fn bar3<T, U, V>(&self, x: usize);
}

struct X;
impl Trait for X {
    reuse FromTrait::foo::<(), ()> as bar;
    reuse FromTrait::foo as bar1;
    reuse FromTrait::foo as bar2;
    reuse FromTrait::<'static, (), ()>::foo::<(), ()> as bar3;

    // Desugaring:
    #[attr = Inline(Hint)]
    fn bar() -> _ { FromTrait::foo::<(), ()>() }

    #[attr = Inline(Hint)]
    fn bar1(self: _) -> _ { FromTrait::foo(self) }

    #[attr = Inline(Hint)]
    fn bar2<T, U>(self: _, arg1: _)
        -> _ { FromTrait::foo::<T, U>(self, arg1) }

    #[attr = Inline(Hint)]
    fn bar3<T, U, V>(self: _, arg1: _)
        -> _ { FromTrait::<'static, (), ()>::foo::<(), ()>(self, arg1) }
}
```

We do not generate explicit `Self` param, as the situation with signature function resolution is the same as in trait impl to free delegation, moreover we ignore parent generics, if they are not specified then we would get an error, so the user should specify them explicitly: `reuse FromTrait::<'static, (), ()>::...`, infers both in child and parent segments are also ignored.

### TraitImpl-to-impl

### Impl-to-free

The same rules as in trait to free delegation are applied here:

```rust
fn foo<'a: 'a, T, const X: usize>() {}
fn foo2(x: &X) {}

struct X;
impl X {
    reuse foo;
    reuse foo::<'static, (), 123> as bar;
    reuse foo2;

    // Desugaring:
    #[attr = Inline(Hint)]
    fn foo<'a, T, const X: _>() -> _ where 'a:'a { foo::<'a, T, X>() }

    #[attr = Inline(Hint)]
    fn bar() -> _ { foo::<'static, (), 123>() }

    #[attr = Inline(Hint)]
    fn foo2(arg0: _) -> _ { foo2(arg0) }
}
```

Note that even if the signature function has `x: &X` first argument which matches the `&Self` type in inherent impl we do not generate it as receiver (`&self`), instead a default argument is generated. 

### Impl-to-trait

Almost same as trait to trait delegation, with one exception that we change the type of receiver to type of the inherent impl ADT:
```rust
trait FromTrait<'a, B, C> {
    fn foo<T, U>(&self, x: usize) {}
}

struct X;
impl X {
    reuse FromTrait::foo;
    reuse FromTrait::<'static, _, _>::foo as foo1;
    reuse FromTrait::foo::<_, ()> as foo2;
    reuse <X as FromTrait>::foo as foo3;

    // Desugaring:
    #[attr = Inline(Hint)]
    fn foo<'a, B, C, T, U>(self: _, arg1: _) -> _ where
        'a:'a { FromTrait::<'a, B, C>::foo::<T, U>(self, arg1) }

    #[attr = Inline(Hint)]
    fn foo1<B, C, T, U>(self: _, arg1: _)
        -> _ { FromTrait::<'static, B, C>::foo::<T, U>(self, arg1) }

    #[attr = Inline(Hint)]
    fn foo2<'a, B, C, T>(self: _, arg1: _) -> _ where
        'a:'a { FromTrait::<'a, B, C>::foo::<T, ()>(self, arg1) }

    #[attr = Inline(Hint)]
    fn foo3<'a, B, C, T, U>(self: _, arg1: _) -> _ where
        'a:'a { <X as FromTrait::<'a, B, C>>::foo::<T, U>(self, arg1) }
}
```

We also do not generate explicit `Self` generic param as in free to trait delegation. Infers are supported both in parent and child segment of the call path.

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

When generating delegation we inherit and generate attributes. More specifically we inherit `must_use` attribute if it is specified on the signature function. The `inline` attribute is generated, only if there is no same attribute present on the delegation. In case of the recursive delegations, the inherited attribute is propagated on the whole chain of recursive delegations and can be overridden somewhere in this chain.

```rust
#[must_use = "reason"]
#[inline]
fn first() {}

reuse first as second;

trait Trait {
    #[inline(always)]
    #[must_use = "overriden_reason"]
    reuse second as third;
}

reuse Trait::third as fourth;
```

Desugaring:

```rust
#[attr = MustUse {reason: "reason"}]
#[attr = Inline(Hint)]
fn first() { }

// `must_use` is inherited, `inline` is generated.
#[attr = MustUse {reason: "reason"}]
#[attr = Inline(Hint)]
fn second() -> _ { first() }

trait Trait {
    // Both `must_use` and `inline` were specified for delegation,
    // so we use user-specified attributes.
    #[attr = Inline(Always)]
    #[attr = MustUse {reason: "overridden_reason"}]
    fn third() -> _ { second() }
}

// `must_use` is inherited, `inline` is generated.
#[attr = MustUse {reason: "overridden_reason"}]
#[attr = Inline(Hint)]
fn fourth<Self>() -> _ { <Self as Trait>::third() }
```

# Unresolved questions

## Resolution of type-relative path during AST -> HIR lowering