## Examples found in rustc/std

TODO: classification

```rust
pub trait HashStable<CTX> {
	fn hash_stable(&self, hcx: &mut CTX, hasher: &mut StableHasher);
}

pub struct Interned<'a, T>(pub &'a T, pub private::PrivateZst);

impl<T, CTX> HashStable<CTX> for Interned<'_, T>
where
T: HashStable<CTX>,
{
	fn hash_stable(&self, hcx: &mut CTX, hasher: &mut StableHasher) {
		self.0.hash_stable(hcx, hasher);
	}
}
```


```rust
impl<T: ?Sized> fmt::Debug for NonNull<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Pointer::fmt(&self.as_ptr(), f)
	}
}

impl fmt::Display for AsciiChar {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<str as fmt::Display>::fmt(self.as_str(), f)
	}
}

```

```rust
pub fn new(error: E) -> Report<E> {
	Self::from(error)
}
```

```rust
pub fn contains<U>(&self, item: &U) -> bool
	where
		Idx: PartialOrd<U>,
		U: ?Sized + PartialOrd<Idx>,
{
	<Self as RangeBounds<Idx>>::contains(self, item)
}
```


```rust
pub fn is<T: Any>(&self) -> bool {
	<dyn Any>::is::<T>(self)
}
```

```rust
pub fn split_array_ref<const M: usize>(&self) -> (&[T; M], &[T]) {
	(&self[..]).split_array_ref::<M>()
}
```


```rust
impl char {
	pub fn decode_utf16<I: IntoIterator<Item = u16>>(iter: I) -> DecodeUtf16<I::IntoIter> {
		super::decode::decode_utf16(iter)
	}
}

```


```rust
impl Iter<???> {
	pub fn as_slice(&self) -> &'a [T] {
		self.make_slice()
	}

	// Self::Iter -> T
	fn next_chunk<const N: usize>(
		&mut self,
	) -> Result<[Self::Item; N], array::IntoIter<Self::Item, N>>
	where
		Self: Sized,
	{
		array::iter_next_chunk(self)
	}
}
```


```rust

CharIndicies<'a> {
	pub fn as_str(&self) -> &'a str {
		self.iter.as_str()
	}
}

```

```rust
impl<'a, P: Pattern<'a>> Split<'a, P> {
	...
	pub fn remainder(&self) -> Option<&'a str> {
		self.0.remainder()
	}
}
```

```rust
impl str {
	pub fn trim_left_matches<'a, P: Pattern<'a>>(&'a self, pat: P) -> &'a str {
		self.trim_start_matches(pat)
	}

	pub fn trim_right_matches<'a, P>(&'a self, pat: P) -> &'a str
		where
		P: Pattern<'a, Searcher: ReverseSearcher<'a>>,
	{
		self.trim_end_matches(pat)
	}
}
```

```rust
impl<T> [T] {
	...
	pub fn fill(&mut self, value: T)
		where
		T: Clone,
	{
		specialize::SpecFill::spec_fill(self, value);
	}

	pub fn is_sorted_by_key<'a, F, K>(&'a self, f: F) -> bool
		where
		F: FnMut(&'a T) -> K,
		K: PartialOrd,
	{
		self.iter().is_sorted_by_key(f)
	}

	pub fn concat<Item: ?Sized>(&self) -> <Self as Concat<Item>>::Output
	where
		Self: Concat<Item>,
	{
		Concat::concat(self)
	}
}

```

```rust
impl<T, const LANES: usize> Mask<T, LANES>
where
	T: MaskElement,
	LaneCount<LANES>: SupportedLaneCount,
{
	pub fn to_int(self) -> Simd<T, LANES> {
		self.0.to_int()
	}
}
```


```rust
impl<T, A: Allocator> RawVec<T, A> {
	pub fn ptr(&self) -> *mut T {
		self.ptr.as_ptr()
	}
}
```

```rust
impl<T: ?Sized> ThinBox<T> {
	fn data(&self) -> *mut u8 {
		self.with_header().value()
	}
}
```

```rust
impl<H> WithHeader<H> {
	const fn header_size() -> usize {
		mem::size_of::<H>()
	}
}
```

```rust

impl<T: ?Sized + Error> Error for ThinBox<T> {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		self.deref().source()
	}
}

```

```rust
impl<T> Box<T> {
	pub fn new(x: T) -> Self {
		#[rustc_box]
		Box::new(x)
	}
}
```

```rust
impl<T, A: Allocator> BinaryHeap<T, A> {
	pub fn capacity(&self) -> usize {
		self.data.capacity()
	}
	...
	pub fn into_vec(self) -> Vec<T, A> {
		self.into()
	}

	pub fn allocator(&self) -> &A {
		self.data.allocator()
	}
}
```

```rust
impl<'a, K: Ord, V, A: Allocator + Clone> OccupiedEntry<'a, K, V, A> {
	pub fn remove_entry(self) -> (K, V) {
		self.remove_kv()
	}
	...
	pub fn into_mut(self) -> &'a mut V {
		self.handle.into_val_mut()
	}
}
```

```rust
impl<T, A: Allocator + Clone> BTreeSet<T, A> {
	pub fn clear(&mut self)
		where
		A: Clone,
	{
		self.map.clear()
	}

	pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
	where
		T: Borrow<Q> + Ord,
		Q: Ord,
	{
		self.map.contains_key(value)
	}

	pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
	where
		T: Borrow<Q> + Ord,
		Q: Ord,
	{
		Recover::get(&self.map, value)
	}
}
```


```rust
pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
	where
	K: Borrow<Q>,
	Q: Hash + Eq,
	{
		self.base.get(k)
	}

```

```rust
impl<K, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, RandomState>
where
	K: Eq + Hash,
{
	fn from(arr: [(K, V); N]) -> Self {
		Self::from_iter(arr)
	}
}
```


```rust
pub struct OccupiedEntry<
	'a,
	T,
	A: Allocator + Clone = Global,
>
{
	pub(super) inner: map::OccupiedEntry<'a, T, SetValZST, A>,
}
```

##### free fn


```rust
reuse<T: ConvertVec, A: Allocator> T::to_vec;

pub fn to_vec<T: ConvertVec, A: Allocator>(s: &[T], alloc: A) -> Vec<T, A> {
	T::to_vec(s, alloc)
}
```

```rust

// Used by `BufReader::spec_read_byte`, for which the `inline(ever)` is
// important.

#[inline(never)]
fn uninlined_slow_read_byte<R: Read>(reader: &mut R) -> Option<Result<u8>> {
	inlined_slow_read_byte(reader)
}

```


```rust


pub fn fix_windows_verbatim_for_gcc(p: &Path) -> PathBuf {
	p.to_path_buf()
}

```

```rust

/// Returns the length of the longest LEB128 encoding for `T`, assuming `T` is an integer type
pub const fn max_leb128_len<T>() -> usize {
// The longest LEB128 encoding for an integer uses 7 bits per byte.
(std::mem::size_of::<T>() * 8 + 6) / 7
}



/// Returns the length of the longest LEB128 encoding of all supported integer types.
pub const fn largest_max_leb128_len() -> usize {
	max_leb128_len::<u128>()
}

```

```rust

pub fn vis_to_string(v: &ast::Visibility) -> String {
	State::new().vis_to_string(v)
}



pub fn meta_list_item_to_string(li: &ast::NestedMetaItem) -> String {
	State::new().meta_list_item_to_string(li)
}



pub fn attribute_to_string(attr: &ast::Attribute) -> String {
	State::new().attribute_to_string(attr)
}



pub fn to_string(f: impl FnOnce(&mut State<'_>)) -> String {
	State::to_string(f)
}

```


```rust
fn stable<F>(name: S, f: F) -> R
where
	F: Fn(&mut getopts::Options) -> &mut getopts::Options + 'static,
	{
		RustcOptGroup::stable(name, f)
	}


fn unstable<F>(name: S, f: F) -> R
where
	F: Fn(&mut getopts::Options) -> &mut getopts::Options + 'static,
{
	RustcOptGroup::unstable(name, f)
}
```

```rust
pub fn stream_to_parser<'a>(
	sess: &'a ParseSess,
	stream: TokenStream,
	subparser_name: Option<&'static str>,
) -> Parser<'a> {
	Parser::new(sess, stream, subparser_name)
}
```

```rust
pub fn combine_substructure(
	f: CombineSubstructureFunc<'_>,
) -> RefCell<CombineSubstructureFunc<'_>> {
	RefCell::new(f)
}
```

```rust
pub fn reachable<'a, 'tcx>(
	body: &'a Body<'tcx>,
) -> impl 'a + Iterator<Item = (BasicBlock, &'a BasicBlockData<'tcx>)> {
	preorder(body)
}
```
