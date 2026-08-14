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

TODO

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

TODO

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

TODO

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

TODO

### Inheriting clauses and signature

We construct delegation signature and clauses after AST -> HIR lowering, during analysis. For this purpose we copy clauses from signature function and its parent if present. Next, user can specify generic arguments in the delegation call path, so we need to consider them while building delegation's signature:

```rust
fn foo<T>(t: T) {}

reuse foo::<usize> as bar;

// Desired desugaring:
fn bar(t: usize) { foo::<usize>(t) }
```

In the example above we want generated delegation to have argument of type `usize` and we do not want to generate generic param `T`. Moreover, if the signature function contains clauses we want map them onto user-specified generic arguments if they are present:

```rust
trait Marker<T> {}
trait Trait<T, U: Marker<T>> {
    fn foo<V>(self, v: V, u: U);
}

reuse Trait::<usize, _>::foo;

// Desugaring:
#[attr = Inline(Hint)]
fn foo<Self, U, V>(self: _, arg1: _, arg2: _)
    -> _ { <Self as Trait::<usize, U>>::foo::<V>(self, arg1, arg2) }
```

In the example above we want `Self: Trait<usize, U>`, `U: Marker<usize>` clauses to be present.

Delegation's parent generic arguments can also be used by delegation:

```rust
trait Marker<T> {}
trait Trait<T, U: Marker<T>> {
    fn foo<V>(self, v: V, u: U);
}

trait Trait2<A, B> {
    reuse Trait::<usize, A>::foo;
}
```

So when mapping clauses and signature we also need to consider delegation's parent generics. Given all that and the possibility of different `Self` positions we construct the following generic arguments and remap table for generics:

```rust
/// [SELF | maybe self in the beginning]
/// [PARENT | args of delegation parent]
/// [SIG PARENT LIFETIMES]
/// [SIG LIFETIMES]
/// [SELF | maybe self after lifetimes, when we reuse trait fn in free context]
/// [SIG PARENT TYPES/CONSTS]
/// [SIG TYPES/CONSTS]
```

Next we iterate over delegation parent, signature parent and child mapping indices of signature parent and child into new generic arguments. Note that we do not have to map delegation's parent indices as they corresponds to indices of generic arguments in new array. For the example above the mapping and new generic args will be:
```rust
[Self/#0, A/#1, B/#2, usize, A/#1, V/#3], {0: 0, 3: 5, 1: 3, 2: 4}
```

So `Self` generic param is mapped into itself, first generic param of trait is mapped into `usize` (index `3` in the new args), `U` is mapped into fourth arg (`A`, from delegation parent) and `V` is mapped into the last argument. Note that delegation's parent `A` and `B` are present in the beginning of new args straight after `Self` arg.

## Recursive delegations

Delegations can be recursive, the recursion chain is defined by the signature function ID.

```rust
trait Trait1 {
    fn foo(&self) {}
}

impl Trait1 for () {}

struct S1<T>(T);
impl<T: Trait1> S1<T> {
    reuse Trait1::foo { self.0 }
}

struct S2(S1<()>);
impl S2 {
    reuse S1::<()>::foo { self.0 }
}

reuse S2::foo;

struct S3;
impl S3 {
    reuse foo;
}

impl Trait1 for S3 {
    reuse S2::foo { &S2(S1(())) }
}

trait Trait2 {
    reuse <S3 as Trait1>::foo { S3 }
}

reuse Trait2::foo as trait_foo;

struct S4;
impl S4 {
    reuse trait_foo;
}

reuse S4::trait_foo as trait_foo_reused;
```

If we encounter cycle in recursive delegations chain then we report an error.

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

One of common case for delegation is to delegate from a newtype that wraps some other type to a trait. In this case we know that `Self` in trait declaration and `Self` in the trait impl would have different types, so we need to perform additional actions to make it compile:

```rust
trait Trait {
    fn method(&self) -> Self;
}

struct S;
impl Trait for S {
    fn method(&self) -> S { S }
}

struct W(S);
impl Trait for W {
    reuse Trait::method { self.0 }

    // Desugaring:
    #[attr = Inline(Hint)]
    fn method(self: _) -> _ { from(Self { 0: Trait::method(self.0) }) }
}
```

In the example above when desugaring `method` in `W` trait impl by default it will return `S` as its signature in trait has `Self` in the return type. However, we implementing this trait for a newtype, so we would like to wrap the return value into `W { 0: /* delegation */ }`, otherwise the code will not work. That is basically what we do when we see that the delegation is in trait impl and the return type of the signature function is something that contains `Self` in generic arguments of the ADT or is inside a reference. The condition is so relaxed, because we can encounter return types such as `Box<Self>` or `Rc<Self>` which require additional processing to work: we wrap the return value of the generated delegation with a single `From::from` call. This solution will not support complex return types such as `Rc<Box<Self>>`, where theoretically we could have generated two `From::from` calls and this will work, however this approach is limited only to pointer types where we know their structure ahead of time, with complex custom type trees it is not obvious what to generate: `Struct<Arc<Rc<Self>>, Self, Self>`, that is why we decided to generate a single `From::from` call. Supporting more complex return value conversions could have involved explicitly specifying them, however that will indirectly (or directly) grow delegation's syntax budget, which we try to avoid.

Next come arguments which also can be mapped. The selector for such arguments is the same as selector for mapping the return value: it contains `Self` generic param and the delegation is in trait impl.

```rust
trait MyAdd {
    fn add(self, other: Self) -> Self;
}

impl MyAdd for usize {
    fn add(self, other: usize) -> usize { self + other }
}

struct W(usize);
reuse impl MyAdd for W { self.0 }

// Desugaring:
#[attr = Inline(Hint)]
fn add(self: _, arg1: _)
    -> _ { from(Self { 0: MyAdd::add(self.0, self.0) }) }
```

In the example above the `other` argument need to be mapped, as if we don't do it then we get code that does not compile. The intuition here is to apply target expression and adjustments not only to the first argument but for all arguments that need to be mapped. Note that we apply adjustments too:
```rust
trait MyAdd {
    fn add(self: &Self, other: &mut Self, another_other: Self);
}

impl MyAdd for usize {
    fn add(self: &Self, other: &mut Self, another_other: Self) {}
}

struct W(Box<Box<Box<usize>>>);
reuse impl MyAdd for W { self.0 }
```

The code above will compile as we will apply adjustments for all arguments of `add` function (as if all arguments were at receiver position and we were processing method call).
Note that as we apply target expression to more than one argument, we need to lower it several times which causes errors when a single `LocalDefId` is lowered more than once. We can re-lower rust code that does not contain definitions, but re-lowering target expressions with definitions is prohibited for now.

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

# Diagnostics

The following errors are reported:

- `failed to resolve delegation callee` - when we failed to resolve delegation,
- `encountered a cycle during delegation signature resolution` - when there is a cycle in the recursive delegations chain,
- `delegation's target expression is specified for function with no params` - when target expression is specified for delegation whose signature function has no params (does not reported in case of list or glob delegations),
- `wrong infer used: expected {$expected}, found: {$actual}` - when `'_` is used instead of `_` and vice versa in user specified generics arguments,
- `attempted to lower target expression with definitions more than once while mapping argument` - when user declared definition in the target expression which is lowered more than once during `Self` type mapping,
- `ambiguous delegation to inherent impl function` - when there are several candidates for delegation to inherent impl,
- `delegation to inherent impl must contain parent generics` - when delegating to inherent impl parent segment of the call path must contain user-specified generic arguments,
- `parent segment of delegation to inherent impl can not contain infers` - infers are prohibited in delegation to inherent impls,
- `UnsupportedDelegation` - when delegation to this entity is not supported,
- `inferred lifetimes are not allowed in delegations as we need to inherit signature` - when we encountered infer regions during delegation signature inheritance,

This was the list of named diagnostics that are reported, if we encounter any other error, we would generate an "error" delegation - a function with no parameters that just calls the call path with lowered target expression, then other parts of the compiler will report real errors which caused errors in delegation generation.

# Unresolved questions

## Resolution of type-relative path during AST -> HIR lowering

As we support (partially for now) delegations to inherent impls, we need to use `ProbeContext` routine to fairly resolve them during AST -> HIR lowering of the delegation (or before AST -> HIR lowering at all). The research of how to solve it was done in this [pull request](https://github.com/rust-lang/rust/pull/155337). TLDR the idea is to execute resolution of delegations in a sandbox: a special mode of the compiler that allows us to forget that delegations exist in the code base and execute resolution through `ProbeContext` without query cycles. That is a large infrastructural work with many questions, like how to prevent leak of already allocated definitions into the sandbox, how to make it performant friendly, how to invalidate query caches after sandbox, should sandbox be executed inplace in the current query system or maybe we should clone it and execute there, etc.

## Removing already allocated `LocalDefId`

When an id for definition is allocated it is hard to remove it with zero perf cost, as data structure in structs like `Definitions` are highly optimized for storage of continuous range of numbers (`IndexVec`) and when it comes to removal it is hard (well at least for me) to (1) implement it in a performant friendly way and (2) prevent leak of definitions id from other sources, for example we have `resolutions(())` query which is in fact a global state we have after resolution, and any developer may put a new map there, fill it with definitions ids that will be deleted during AST -> HIR lowering and then it will be accessed in some other stage of the compiler, which will think that it is not deleted, while other parts of the compiler will think that this definition is deleted.

## Lower single target expression with definitions inside twice during AST -> HIR lowering

Glob and list delegations are expanded during macro expansion, meaning we do not have full resolution information, in particular we do not know which arguments need to be mapped (recall the `Self` type mapping section). If we had this information we could generate several target expressions and thus we would not try to lower single target expression more than once. As another way we could clone this target expression and all resolution results that are connected to it and lower a deep copy of it. However, it may require altering outputs of resolution-level queries (as we deep cloned target expression we allocated new node ids and definitions ids, so for all other code to work we need to put this information alongside all other information from the resolve stage), such as `resolutions(())`, which is impossible right now.