# MiniRust representation relation

The main purpose of [types](types.md) is to define how [values](values.md) are (de)serialized into/from memory.
This is the *[representation relation]*, which is defined in the following.
`decode` converts a list of bytes into a value; this operation can fail if the byte list is not a valid encoding for the given type.
`encode` inverts `decode`; it will always work when the value is [well-formed][well-formed-value] for the given type (which the specification must ensure, i.e. violating this property is a spec bug).

[representation relation]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/reference/src/glossary.md#representation-relation
[well-formed-value]: well-formed.md#well-formed-values

## Type-directed Encode/Decode of values

The definition of these functions is huge, so we split it by type.
(We basically pretend we can have fallible patterns for the `self` parameter and declare the function multiple times with non-overlapping patterns.
If any pattern is not covered, that is a bug in the spec.)

```rust
impl Type {
    /// Decode a list of bytes into a value. This can fail, which typically means Undefined Behavior.
    /// `decode` must satisfy the following property:
    /// ```
    /// ty.decode(bytes) = Some(_) -> bytes.len() == ty.size() && ty.inhabited()`
    /// ```
    /// In other words, all valid low-level representations must have the length given by the size of the type,
    /// and the existence of a valid low-level representation implies that the type is inhabited.
    #[specr::argmatch(self)]
    fn decode<M: Memory>(self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> { .. }

    /// Encode `v` into a list of bytes according to the type `self`.
    /// Note that it is a spec bug if `v` is not well-formed for `ty`!
    ///
    /// See below for the general properties relation `encode` and `decode`.
    #[specr::argmatch(self)]
    fn encode<M: Memory>(self, val: Value<M>) -> List<AbstractByte<M::Provenance>> { .. }
}
```

TODO: We currently have `encode` panic when the value doesn't match the type.
Should we also have `decode` panic when `bytes` has the wrong length?

### `bool`

```rust
impl Type {
    fn decode<M: Memory>(Type::Bool: Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        if bytes.len() != 1 {
            throw!();
        }
        ret(match bytes[0] {
            AbstractByte::Init(0, _) => Value::Bool(false),
            AbstractByte::Init(1, _) => Value::Bool(true),
            _ => throw!(),
        })
    }
    fn encode<M: Memory>(Type::Bool: Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        let Value::Bool(b) = val else { panic!() };
        list![AbstractByte::Init(if b { 1 } else { 0 }, None)]
    }
}
```

Note, in particular, that `bool` just entirely ignored provenance; we discuss this a bit more when we come to integer types.

### Integers

```rust
impl Type {
    fn decode<M: Memory>(Type::Int(IntType { signed, size }): Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        if bytes.len() != size.bytes() {
            throw!();
        }
        // Fails if any byte is `Uninit`.
        let bytes_data = bytes.try_map(|b| b.data())?;
        ret(Value::Int(M::T::ENDIANNESS.decode(signed, bytes_data)))
    }
    fn encode<M: Memory>(Type::Int(IntType { signed, size }): Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        let Value::Int(i) = val else { panic!() };
        // `Endianness::encode` will do the integer's bound check.
        let bytes_data = M::T::ENDIANNESS.encode(signed, size, i).unwrap();
        bytes_data.map(|b| AbstractByte::Init(b, None))
    }
}
```

This entirely ignores provenance during decoding, and generates `None` provenance during encoding.
This corresponds to having ptr-to-int transmutation implicitly strip provenance (i.e., it behaves like [`addr`](https://doc.rust-lang.org/nightly/std/primitive.pointer.html#method.addr)),
and having int-to-ptr transmutation generate "invalid" pointers (like [`ptr::invalid`](https://doc.rust-lang.org/nightly/std/ptr/fn.invalid.html)).
This is required to achieve a "monotonicity" with respect to provenance (as discussed [below](#generic-properties)).

- TODO: Is that the right semantics for ptr-to-int transmutation? See [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/286).
- TODO: This does not allow uninitialized integers. I think that is fairly clearly what we want, also considering LLVM is moving towards using `noundef` heavily to avoid many of the current issues in their `undef` handling. But this is also still [being discussed](https://github.com/rust-lang/unsafe-code-guidelines/issues/71).

### Pointers

```rust
fn decode_ptr<M: Memory>(bytes: List<AbstractByte<M::Provenance>>) -> Option<Pointer<M::Provenance>> {
    if bytes.len() != M::T::PTR_SIZE.bytes() { throw!(); }
    // Convert into list of bytes; fail if any byte is uninitialized.
    let bytes_data = bytes.try_map(|b| b.data())?;
    let addr = M::T::ENDIANNESS.decode(Unsigned, bytes_data);
    // Get the provenance. Must be the same for all bytes, else we use `None`.
    let mut provenance: Option<M::Provenance> = bytes[0].provenance();
    for b in bytes {
        if b.provenance() != provenance {
            provenance = None;
        }
    }
    ret(Pointer { addr, provenance })
}

fn encode_ptr<M: Memory>(ptr: Pointer<M::Provenance>) -> List<AbstractByte<M::Provenance>> {
    let bytes_data = M::T::ENDIANNESS.encode(Unsigned, M::T::PTR_SIZE, ptr.addr).unwrap();
    bytes_data.map(|b| AbstractByte::Init(b, ptr.provenance))
}

impl Type {
    fn decode<M: Memory>(Type::Ptr(ptr_type): Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        let ptr = decode_ptr::<M>(bytes)?;
        match ptr_type {
            PtrType::Raw | PtrType::FnPtr => {}, // nothing to check
            PtrType::Ref { pointee, mutbl: _ } | PtrType::Box { pointee } => {
                // References (and `Box`) need to be non-null, aligned, and not point to an uninhabited type.
                // (Think: uninhabited types have impossible alignment.)
                ensure(ptr.addr != 0 && ptr.addr % pointee.align.bytes() == 0 && pointee.inhabited)?;
            }
        }
        ret(Value::Ptr(ptr))
    }
    fn encode<M: Memory>(Type::Ptr(_): Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        let Value::Ptr(ptr) = val else { panic!() };
        encode_ptr::<M>(ptr)
    }
}
```

Note that types like `&!` have no valid representation:
when the pointee type is uninhabited (in the sense of `!ty.inhabited()`), there exists no valid reference to that type.

- TODO: This definition says that when multiple provenances are mixed, the pointer has `None` provenance, i.e., it is "invalid".
  Is that the semantics we want? Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/286#issuecomment-1136948796).
- TODO: Do we really want to special case references to uninhabited types? Do we somehow want to require more, like pointing to a valid instance of the pointee type?
  (The latter would not even be possible with the current structure of MiniRust.)
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/77).

### Tuples (and structs, ...)

```rust
impl Type {
    fn decode<M: Memory>(Type::Tuple { fields, size }: Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        if bytes.len() != size.bytes() { throw!(); }
        ret(Value::Tuple(
            fields.try_map(|(offset, ty)| {
                let subslice = bytes.subslice_with_length(offset.bytes(), ty.size::<M::T>().bytes());
                ty.decode::<M>(subslice)
            })?
        ))
    }
    fn encode<M: Memory>(Type::Tuple { fields, size }: Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        let Value::Tuple(values) = val else { panic!() };
        assert_eq!(values.len(), fields.len());
        let mut bytes = list![AbstractByte::Uninit; size.bytes()];
        // FIXME: can we do this in a single mutation, and avoid creating all those temporary objects?
        for ((offset, ty), value) in fields.zip(values) {
            bytes.write_subslice_at_index(offset.bytes(), ty.encode::<M>(value));
        }
        bytes
    }
}
```

Note in particular that `decode` ignores the bytes which are before, between, or after the fields (usually called "padding").
`encode` in turn always and deterministically makes those bytes `Uninit`.
(The [generic properties](#generic-properties) defined below make this the only possible choice for `encode`.)

### Arrays

```rust
impl Type {
    fn decode<M: Memory>(Type::Array { elem, count }: Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        let elem_size = elem.size::<M::T>();
        let full_size = elem_size * count;

        if bytes.len() != full_size.bytes() { throw!(); }

        let chunks: List<_> = (Int::ZERO..count).map(|i|
            bytes.subslice_with_length(i*elem_size.bytes(), elem_size.bytes())
        ).collect();

        ret(Value::Tuple(
            chunks.try_map(|elem_bytes| elem.decode::<M>(elem_bytes))?
        ))
    }
    fn encode<M: Memory>(Type::Array { elem, count }: Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        let Value::Tuple(values) = val else { panic!() };
        assert_eq!(values.len(), count);
        values.flat_map(|value| {
            let bytes = elem.encode::<M>(value);
            assert_eq!(bytes.len(), elem.size::<M::T>().bytes());
            bytes
        })
    }
}
```

### Unions

A union simply stores the bytes directly, no high-level interpretation of data happens.

- TODO: Some real unions actually do not preserve all bytes, they [can have padding](https://github.com/rust-lang/unsafe-code-guidelines/issues/156).
  So we have to model that there can be "gaps" between the parts of the byte list that are preserved perfectly.
- TODO: Should we require *some* kind of validity? See [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/73).

```rust
impl Type {
    fn decode<M: Memory>(Type::Union { size, chunks, .. }: Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        if bytes.len() != size.bytes() { throw!(); }
        let mut chunk_data = list![];
        // Store the data from each chunk.
        for (offset, size) in chunks {
            chunk_data.push(bytes.subslice_with_length(offset.bytes(), size.bytes()));
        }
        ret(Value::Union(chunk_data))
    }
    fn encode<M: Memory>(Type::Union { size, chunks, .. }: Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        let Value::Union(chunk_data) = val else { panic!() };
        assert_eq!(chunk_data.len(), chunks.len());
        let mut bytes = list![AbstractByte::Uninit; size.bytes()];
        // Restore the data from each chunk.
        // FIXME: can we do this in a single mutation, and avoid creating all those temporary objects?
        for ((offset, size), data) in chunks.zip(chunk_data) {
            assert_eq!(size.bytes(), data.len());
            bytes.write_subslice_at_index(offset.bytes(), data);
        }
        bytes
    }
}
```

### Enums

TODO: implement Enum decoding & encoding.

```rust
impl Type {
    fn decode<M: Memory>(Type::Enum { .. }: Self, bytes: List<AbstractByte<M::Provenance>>) -> Option<Value<M>> {
        todo!()
    }

    fn encode<M: Memory>(Type::Enum { .. }: Self, val: Value<M>) -> List<AbstractByte<M::Provenance>> {
        todo!()
    }
}
```

## Generic properties

There are some generic properties that `encode` and `decode` must satisfy.
The most obvious part is consistency of size and inhabitedness:
- If `ty.decode(bytes) == Some(val)`, then `bytes` has length `ty.size()` and `ty.inhabited() == true`.

More interestingly, we have some round-trip properties.
For instance, starting with a (well-formed) value, encoding it, and then decoding it, must produce the same result.

To make this precise, we first have to define an order in values and byte lists that captures when one value (byte list) is "more defined" than another.
"More defined" here can either mean initializing some previously uninitialized data, or adding provenance to data that didn't have it.
(Adding provenance means adding the permission to access some memory, so this can make previously undefined programs defined, but it can never make previously defined programs undefined.)

Note that none of the definitions in this section are needed to define the semantics of a Rust program, or to make MiniRust into a runnable interpreter.
They only serve as internal consistency requirements of the semantics.
It would be a specification bug if the representation relations defined above violated these properties.

```rust
trait DefinedRelation {
    /// returns whether `self` is less or as defined as `other`
    fn le_defined(self, other: Self) -> bool;
}
```

Starting with `AbstractByte`, we define `b1 <= b2` ("`b1` is less-or-equally-defined as `b2`") as follows:

```rust
impl<Provenance> DefinedRelation for AbstractByte<Provenance> {
    fn le_defined(self, other: Self) -> bool {
        use AbstractByte::*;
        match (self, other) {
            // `Uninit <= _`: initializing something makes it "more defined".
            (Uninit, _) =>
                true,
            // Among initialized bytes, adding provenance makes it "more defined".
            (Init(data1, None), Init(data2, _)) =>
                data1 == data2,
            // If both bytes have provenance, everything must be equal.
            (Init(data1, Some(provenance1)), Init(data2, Some(provenance2))) =>
                data1 == data2 && provenance1 == provenance2,
            // Nothing else is related.
            _ => false,
        }
    }
}
```

Similarly, on `Pointer` we say that adding provenance makes it more defined:
```rust
impl<Provenance> DefinedRelation for Pointer<Provenance> {
    fn le_defined(self, other: Self) -> bool {
        self.addr == other.addr &&
            match (self.provenance, other.provenance) {
                (None, _) => true,
                (Some(prov1), Some(prov2)) => prov1 == prov2,
                _ => false,
            }
    }
}
```

The order on `List<AbstractByte>` is assumed to be such that `bytes1 <= bytes2` if and only if they have the same length and are bytewise related by `<=`.
In fact, we define this to be in general how lists are partially ordered (based on the order of their element type):
```rust
impl<T: DefinedRelation> DefinedRelation for List<T> {
    fn le_defined(self, other: Self) -> bool {
        self.len() == other.len() &&
            self.zip(other).all(|(l, r)| l.le_defined(r))
    }
}
```

For `Value`, we lift the order on byte lists to relate `Bytes`s, and otherwise require equality:
```rust
impl<M: Memory> DefinedRelation for Value<M> {
    fn le_defined(self, other: Self) -> bool {
        use Value::*;
        match (self, other) {
            (Int(i1), Int(i2)) =>
                i1 == i2,
            (Bool(b1), Bool(b2)) =>
                b1 == b2,
            (Ptr(p1), Ptr(p2)) =>
                p1.le_defined(p2),
            (Tuple(vals1), Tuple(vals2)) =>
                vals1.le_defined(vals2),
            (Variant { idx: idx1, data: data1 }, Variant { idx: idx2, data: data2 }) =>
                idx1 == idx2 && data1.le_defined(data2),
            (Union(chunks1), Union(chunks2)) => chunks1.le_defined(chunks2),
            _ => false
        }
    }
}
```

Finally, on `Option<Value>` we assume that `None <= _`, and `Some(v1) <= Some(v2)` if and only if `v1 <= v2`:
```rust
impl<T: DefinedRelation> DefinedRelation for Option<T> {
    fn le_defined(self, other: Self) -> bool {
        match (self, other) {
            (None, _) => true,
            (Some(l), Some(r)) => l.le_defined(r),
            _ => false
        }
    }
}
```

In the following, let `ty` be an arbitrary well-formed type.
We say that a `v: Value` is ["well-formed"][well-formed-value] for a type if `v.check_wf(ty)` is `Some(_)`.
This ensures that the basic structure of the value and the type match up.
Decode will only ever return well-formed values.

Now we can state the laws that we require.
First of all, `encode` and `decode` must both be "monotone":
- If `val1 <= val2` (and if both values are well-formed for `ty`), then `ty.encode(val1) <= ty.encode(val2)`.
- If `bytes1 <= bytes2`, then `ty.decode(bytes1) <= ty.decode(bytes2)`.

More interesting are the round-trip properties:
- If `val` is well-formed for `ty`, then `ty.decode(ty.encode(val)) == Some(val)`.
  In other words, encoding a value and then decoding it again is lossless.
- If `ty.decode(bytes) == Some(val)`, then `ty.encode(val) <= bytes`.
  In other words, if a byte list is successfully decoded, then encoding it again will lead to a byte list that is "less defined"
  (some bytes might have become `Uninit`, but otherwise it is the same).

(For the category theory experts: this is called an "adjoint" relationship, or a "Galois connection" in abstract interpretation speak.
Monotonicity ensures that `encode` and `decode` are functors.)

The last property might sound surprising, but consider what happens for padding: `encode` will always make it `Uninit`,
so a bytes-value-bytes roundtrip of some data with padding will reset some bytes to `Uninit`.

Together, these properties ensure that it is okay to optimize away a self-assignment like `tmp = x; x = tmp`.
The effect of this assignment (as defined [later](step.md)) is to decode the `bytes1` stored at `x`, and then encode the resulting value again into `bytes2` and store that back.
(We ignore the intermediate storage in `tmp`.)
The second round-trip property ensures that `bytes2 <= bytes1`.
If we remove the assignment, `x` ends up with `bytes1` rather than `bytes2`; we thus "increase memory" (as in, the memory in the transformed program is "more defined" than the one in the source program).
According to monotonicity, "increasing" memory can only ever lead to "increased" decoded values.
For example, if the original program later did a successful decode at an integer to some `v: Value`, then the transformed program will return *the same* value (since `<=` on `Value::Int` is equality).

## Typed memory accesses

One key use of the value representation is to define a "typed" interface to memory.
This interface is inspired by [Cerberus](https://www.cl.cam.ac.uk/~pes20/cerberus/).
We also use this to lift retagging from pointers to compound values.

```rust
impl<M: Memory> AtomicMemory<M> {
    fn typed_store(&mut self, ptr: Pointer<M::Provenance>, val: Value<M>, pty: PlaceType, atomicity: Atomicity) -> Result {
        let bytes = pty.ty.encode::<M>(val);
        self.store(ptr, bytes, pty.align, atomicity)?;

        ret(())
    }

    fn typed_load(&mut self, ptr: Pointer<M::Provenance>, pty: PlaceType, atomicity: Atomicity) -> Result<Value<M>> {
        let bytes = self.load(ptr, pty.ty.size::<M::T>(), pty.align, atomicity)?;
        ret(match pty.ty.decode::<M>(bytes) {
            Some(val) => {
                assert!(val.check_wf(pty.ty).is_some(), "decode returned {val:?} which is ill-formed for {:#?}", pty.ty);
                val
            }
            None => throw_ub!("load at type {pty:?} but the data in memory violates the validity invariant"), // FIXME use Display instead of Debug for `pty`
        })
    }

    fn retag_val(&mut self, val: Value<M>, ty: Type, fn_entry: bool) -> Result<Value<M>> {
        ret(match (val, ty) {
            // no (identifiable) pointers
            (Value::Int(..) | Value::Bool(..) | Value::Union(..), _) => val,
            // base case
            (Value::Ptr(ptr), Type::Ptr(ptr_type)) => Value::Ptr(self.retag_ptr(ptr, ptr_type, fn_entry)?),
            // recurse into tuples/arrays/enums
            (Value::Tuple(vals), Type::Tuple { fields, .. }) =>
                Value::Tuple(vals.zip(fields).try_map(|(val, (_offset, ty))| self.retag_val(val, ty, fn_entry))?),
            (Value::Tuple(vals), Type::Array { elem: ty, .. }) =>
                Value::Tuple(vals.try_map(|val| self.retag_val(val, ty, fn_entry))?),
            (Value::Variant { idx, data }, Type::Enum { variants, .. }) =>
                Value::Variant { idx, data: self.retag_val(data, variants[idx], fn_entry)? },
            _ => panic!("this value does not have that type"),
        })
    }
}
```

## Relation to validity invariant

One way we *could* also use the value representation (and the author thinks this is exceedingly elegant) is to define the validity invariant.
Certainly, it is the case that if a list of bytes is not related to any value for a given type `T`, then that list of bytes is *invalid* for `T` and it should be UB to produce such a list of bytes at type `T`.
We could decide that this is an "if and only if", i.e., that the validity invariant for a type is exactly "must be in the value representation":

```rust
#[allow(unused)]
fn bytes_valid_for_type<M: Memory>(ty: Type, bytes: List<AbstractByte<M::Provenance>>) -> Result {
    if ty.decode::<M>(bytes).is_none() {
        throw_ub!("data violates validity invariant of type {ty:?}"); // FIXME use Display instead of Debug for `ty`
    }

    ret(())
}
```

Note that there is a second, different, kind of validity invariant:
the invariant satisfied by any possible *encoding* of a value of a given type.
The way things are defined above, `encode` is more strict than `decode` (in the sense that there are valid inputs to `decode` that `encode` will never produce).
For example, `encode` makes padding between struct fields always `Uninit`, but `decode` accepts *any* data there.
So after a typed assignment, the compiler can actually know that this stricter kind of validity is satisfied.
The programmer, on the other hand, only ever has to ensure the weaker kind of validity defined above.

For many types this is likely what we will do anyway (e.g., for `bool` and `!` and `()` and integers), but for references, this choice would mean that *validity of the reference cannot depend on what memory looks like*---so "dereferenceable" and "points to valid data" cannot be part of the validity invariant for references.
The reason this is so elegant is that, as we have seen above, a "typed copy" already very naturally is UB when the memory that is copied is not a valid representation of `T`.
This means we do not even need a special clause in our specification for the validity invariant---in fact, the term does not even have to appear in the specification---as everything juts falls out of how a "typed copy" applies the value representation twice.

## Validity of pointers

For pointers, we often want properties that go a bit beyond what can be encoded in the representation relation.
For instance, we want to ensure references and boxes are dereferenceable.
This does not apply at each and every typed copy (so maybe it shouldn't be called "validity"), but at least when constructing a reference (via `AddrOf`) or when using it (via `Deref`), these things should be true.

```rust
impl<M: Memory> AtomicMemory<M> {
    fn check_pointer_dereferenceable(&self, ptr: Pointer<M::Provenance>, ptr_ty: PtrType) -> Result {
        if let PtrType::Ref { pointee, .. } | PtrType::Box { pointee, .. } = ptr_ty {
            self.dereferenceable(ptr, pointee)?;
        }
        ret(())
    }
}
```

We expect retagging to do *at least* this check as well.

## Transmutation

The representation relation also says everything there is to say about "transmutation".
By this I mean not just the `std::mem::transmute` function, but any operation that "re-interprets data from one type at another type"
(essentially a `reinterpret_cast` in C++ terms).
Transmutation means taking a value at some type, encoding it, and then decoding it *at a different type*.
More precisely:

```rust
/// Transmutes `val` from `type1` to `type2`.
#[allow(unused)]
fn transmute<M: Memory>(val: Value<M>, type1: Type, type2: Type) -> Option<Value<M>> {
    let bytes = type1.encode::<M>(val);
    ret(type2.decode::<M>(bytes)?)
}
```

This operation can, of course, fail, which means that the encoding of `val` is not valid at `type2`.

[Stacked Borrows]: stacked-borrows.md
