# MiniRust Values

The purpose of this file is to describe what the set of *all possible values* is in MiniRust, and how they are represented in memory.
This is one of the key definitions in MiniRust.
The representation relation relates values with lists of [abstract bytes](../mem/interface.md#abstract-bytes):
it defines, for a given value and list of bytes, whether that value is represented by that list.

[representation]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/reference/src/glossary.md#representation
[memory-interface]: memory-interface.md

## Values

The MiniRust value domain is described by the following type definition.

```rust
/// A helper struct containing a pointer with an optional pointer-sized metadata.
/// - For sized types, `meta` is be `None`.
/// - For slice types, `meta` is the length of the slice.
#[derive(PartialEq, Eq)]
struct PointerRepr {
    ptr: Pointer,
    addr: Option<BigInt>,
}

#[derive(PartialEq, Eq)]
enum Value {
    /// A mathematical integer, used for `i*`/`u*` types.
    Int(BigInt),
    /// A Boolean value, used for `bool`.
    Bool(bool),
    /// A pointer value, with an optional metadata, used for references and raw pointers.
    Ptr(PointerRepr),
    /// An n-tuple, used for arrays, structs, tuples (including unit).
    Tuple(List<Value>),
    /// A variant of a sum type, used for enums.
    Variant {
        idx: BigInt,
        data: Value,
    },
    /// Unions are represented as "lists of chunks", where each chunk is just a raw list of bytes.
    Union(List<List<AbstractByte>>),
}
```

The point of this type is to capture the mathematical concepts that are represented by the data we store in memory.
The definition is likely incomplete, and even if it was complete now, we might expand it as Rust grows.
That is okay; all previously defined representation relations are still well-defined when the domain grows, the newly added values will just not be valid for old types as one would expect.

## Representation relation

The main purpose of types is to define how [values](values.md) are (de)serialized from memory.
This is defined in the following.
`decode` converts a list of bytes into a value; this operation can fail if the byte list is not a valid encoding for the given type.
`encode` inverts `decode`; it will always work when the value is valid for the given type (which the specification must ensure, i.e. violating this property is a spec bug).

The definition of these functions is huge, so we split it by type.
(We basically pretend we can have fallible patterns for the `self` parameter and declare the function multiple times with non-overlapping patterns.
If any pattern is not covered, that is a bug in the spec.)

```rust
impl Type {
    /// Decode a list of bytes into a value. This can fail, which typically means Undefined Behavior.
    /// `decode` must satisfy the following property:
    /// ```
    /// type.decode(bytes) = Some(_) -> bytes.len() == type.size() && type.inhabited()`
    /// ```
    /// In other words, all valid low-level representations must have the length given by the size of the type,
    /// and the existence of a valid low-level representation implies that the type is inhabited.
    fn decode(self, bytes: List<AbstractByte>) -> Option<Value>;

    /// Encode `v` into a list of bytes according to the type `self`.
    /// Note that it is a spec bug if `v` is not valid according to `ty`!
    ///
    /// See below for the general properties relation `encode` and `decode`.
    fn encode(self, v: Value) -> List<AbstractByte>;
}
```

- TODO: Define this for the other types.

### `bool`

```rust
impl Type {
    fn decode(Bool: Self, bytes: List<AbstractByte>) -> Option<Value> {
        match *bytes {
            [AbstractByte::Init(0, _)] => Value::Bool(false),
            [AbstractByte::Init(1, _)] => Value::Bool(true),
            _ => throw!(),
        }
    }
    fn encode(Bool: Self, val: Value) -> List<AbstractByte> {
        let Value::Bool(b) = val else { panic!() };
        [AbstractByte::Init(if b { 1 } else { 0 }, None)]
    }
}
```

Note, in particular, that `bool` just entirely ignored provenance; we discuss this a bit more when we come to integer types.

### Integers

For now we only define `u16` and `i16`.

```rust
impl Type {
    fn decode(Int(IntType { signed, size: Size::from_bits(16) }): Self, bytes: List<AbstractByte>) -> Option<Value> {
        let [AbstractByte::Init(b0, _), AbstractByte::Init(b1, _)] = *bytes else { throw!() };
        Value::Int(ENDIANESS.decode(signed, [b0, b1]))
    }
    fn encode(Int(IntType { signed, size: Size::from_bits(16) }): Self, val: Value) -> List<AbstractByte> {
        let Value::Int(i) = val else { panic!() };
        let [b0, b1] = ENDIANESS.encode(signed, i).unwrap();
        [AbstractByte::Init(b0, None), AbstractByte::Init(b1, None)]
    }
}
```

This entirely ignores provenance during decoding, and generates `None` provenance during encoding.
This corresponds to having ptr-to-int transmutation implicitly strip provenance (i.e., it behaves like [`addr`](https://doc.rust-lang.org/nightly/std/primitive.pointer.html#method.addr)),
and having int-to-ptr transmutation generate "invalid" pointers (like [`ptr::invalid`](https://doc.rust-lang.org/nightly/std/ptr/fn.invalid.html)).
This is required to achieve a "monotonicity" with respect to provenance (as discussed [below](#generic-properties)).

- TODO: Is that the right semantics for ptr-to-int transmutation? See [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/286).
- TODO: This does not allow uninitialized integers. I think that is fairly clearly what we want, also considering LLVM is moving towards using `noundef` heavily to avoid many of the current issues in their `undef` handling. But this is also still [being discussed](https://github.com/rust-lang/unsafe-code-guidelines/issues/71).

### Raw pointers

Decoding pointers is a bit inconvenient since we do not know `PTR_SIZE`.

```rust
fn decode_ptr(bytes: List<AbstractByte>, is_unsized: bool) -> Option<PointerRepr> {
    match (bytes.len(), is_unsized) {
        (PTR_SIZE, false) => {}
        (2 * PTR_SIZE, true) => {}
        _ => throw!(),
    }
    // Convert into list of bytes; fail if any byte is uninitialized.
    let bytes_data: [u8; PTR_SIZE] = bytes.map(|b| b.data()).collect()?;
    let addr = ENDIANESS.decode(Unsigned, &bytes_data);
    // Get the provenance. Must be the same for all bytes, else we use `None`.
    let mut provenance: Option<Provenance> = bytes[0].provenance();
    for b in bytes[..PTR_SIZE] {
        if b.provenance() != provenance {
            provenance = None;
        }
    }
    let ptr = Pointer { addr, provenance };
    let meta = is_unsized.then(|| ENDIANESS.decode(Unsigned, bytes[PTR_SIZE..]));
    PointerRepr { ptr, meta }
}

fn encode_ptr(PointerRepr { ptr, meta }: PointerRepr) -> List<AbstractByte> {
    let bytes_data: [u8; PTR_SIZE] = ENDIANESS.encode(Unsigned, ptr.addr).unwrap();
    bytes_data
        .map(|b| AbstractByte::Init(b, ptr.provenance))
        .chain(meta.map_or(list![], |meta| ENDIANESS.encode(Unsigned, meta))
        .collect()
}

impl Type {
    fn decode(RawPtr: Self, bytes: List<AbstractByte>) -> Option<Value> {
        Value::Ptr(decode_ptr(bytes, false)?)
    }
    fn encode(RawPtr: Self, val: Value) -> List<AbstractByte> {
        let Value::Ptr(ptr) = val else { panic!() };
        encode_ptr(ptr)
    }
}
```

- TODO: This definition says that when multiple provenances are mixed, the pointer has `None` provenance, i.e., it is "invalid".
  Is that the semantics we want? Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/286#issuecomment-1136948796).
- TODO: Unsized raw pointers are not supported yet.

### References and `Box`

```rust
/// Check if the given pointer is valid for safe pointer types (`Ref`, `Box`).
fn check_safe_ptr(PointerRepr { ptr, .. }: PointerRepr, pointee: Layout) -> bool {
    // References (and `Box`) need to be non-null, aligned, and not point to an uninhabited type.
    // (Think: uninhabited types have impossible alignment.)
    ptr.addr != 0 && ptr.addr % pointee.align.bytes() == 0 && pointee.inhabited
}

impl Type {
    fn decode(Ref { pointee, .. } | Box { pointee }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        let ptr = decode_ptr(bytes)?;
        if !check_safe_ptr(ptr, pointee) { throw!(); }
        Value::Ptr(ptr)
    }
    fn encode(Ref { .. } | Box { .. }: Self, val: Value) -> List<AbstractByte> {
        let Value::Ptr(ptr) = val else { panic!() };
        encode_ptr(ptr)
    }
}
```

Note that types like `&!` have no valid value: when the pointee type is uninhabited (in the sense of `!ty.inhabited()`), there exists no valid reference to that type.

- TODO: Do we really want to special case references to uninhabited types? Do we somehow want to require more, like pointing to a valid instance of the pointee type?
  (The latter would not even be possible with the current structure of MiniRust.)
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/77).

### Tuples (and structs, ...)

```rust
impl Type {
    fn decode(Tuple { fields, size }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        if bytes.len() != size { throw!(); }
        Value::Tuple(
            fields.into_iter().map(|(offset, ty)| {
                ty.decode(bytes[offset..][..ty.size()])
            }).try_collect()?,
        )
    }
    fn encode(Tuple { fields, size }: Self, val: Value) -> List<AbstractByte> {
        let Value::Tuple(values) = val else { panic!() };
        let mut bytes = list![AbstractByte::Uninit; size];
        for ((offset, ty), value) in fields.into_iter().zip(values) {
            bytes[offset..][..ty.size()].copy_from_slice(ty.encode(value));
        }
        bytes
    }
}
```

Note in particular that `decode` ignores the bytes which are before, between, or after the fields (usually called "padding").
`encode` in turn always and deterministically makes those bytes `Uninit`.
(The [generic properties](#generic-properties) defined below make this the only possible choice for `encode`.)

### Arrays and slices

```rust
fn decode_slice(elem_ty: Type, len: Option<Size>, bytes: List<AbstractByte>) -> Option<Value> {
    match len {
        Some(len) if bytes.len() != elem_ty.size() * len => throw!(),
        // TODO: handle zero-sized types correctly.
        None if bytes.len() % elem_ty.size() => throw!(),
        _ => {}
    }
    Value::Tuple(
        bytes.chunks(elem_ty.size())
            .map(|elem_bytes| elem.decode(elem_bytes))
            .try_collect()?,
    )
}

fn encode_slice(elem_ty: Type, len: Option<Size>, values: List<Value>) -> List<AbstractByte> {
    if let Some(len) = len {
        assert_eq!(values.len(), len);
    }
    values.into_iter().flat_map(|value| {
        let bytes = elem_ty.encode(value);
        assert_eq!(bytes.len(), elem_ty.size());
        bytes
    }).collect()
}

impl Type {
    fn decodeArray { elem, count }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        decode_slice(elem, Some(count), bytes)
    }
    fn decode(Slice(elem): Self, bytes: List<AbstractByte>) -> Option<Value> {
        decode_slice(elem, None, bytes)
    }

    fn encode(Array { elem, count }: Self, val: Value) -> List<AbstractByte> {
        let Value::Tuple(values) = val else { panic!() };
        encode_slice(elem, Some(count), values)
    }
    fn encode(Slice(elem): Self, val: Value) -> List<AbstractByte> {
        let Value::Tuple(values) = val else { panic!() };
        encode_slice(elem, None, values)
    }
}
```

- TODO: Should we consider paddings between two adjacent elements in arrays (or slices)?

### Unions

A union simply stores the bytes directly, no high-level interpretation of data happens.

- TODO: Some real unions actually do not preserve all bytes, they [can have padding](https://github.com/rust-lang/unsafe-code-guidelines/issues/156).
  So we have to model that there can be "gaps" between the parts of the byte list that are preserved perfectly.
- TODO: Should we require *some* kind of validity? See [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/73).

```rust
impl Type {
    fn decode(Union { size, chunks, .. }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        if bytes.len() != size { throw!(); }
        let mut chunk_data = list![];
        // Store the data from each chunk.
        for (offset, size) in chunks {
            chunk_data.push(bytes[offset..][..size]);
        }
        Value::Union(chunk_data)
    }
    fn encode(Union { size, chunks, .. }: Self, value: Value) -> List<AbstractByte> {
        let Value::Union(chunk_data) = val else { panic!() };
        assert_eq!(chunk_data.len(), chunks.len());
        let mut bytes = [AbstractByte::Uninit; size];
        // Restore the data from each chunk.
        for ((offset, size), data) in chunks.iter().zip(chunk_data.iter()) {
            assert_eq!(data.len(), size);
            bytes[offset..][..size] = data;
        }
        bytes
    }
}
```

### Generic properties

There are some generic properties that `encode` and `decode` must satisfy.
For instance, starting with a (valid) value, encoding it, and then decoding it, must produce the same result.

To make this precise, we first have to define an order in values and byte lists that captures when one value (byte list) is "more defined" than another.
"More defined" here can either mean initializing some previously uninitialized data, or adding provenance to data that didn't have it.
(Adding provenance means adding the permission to access some memory, so this can make previously undefined programs defined, but it can never make previously defined programs undefined.)

Note that none of the definitions in this section are needed to define the semantics of a Rust program, or to make MiniRust into a runnable interpreter.
They only serve as internal consistency requirements of the semantics.
It would be a specification bug if the representation relations defined above violated these properties.

Starting with `AbstractByte`, we define `b1 <= b2` ("`b1` is less-or-equally-defined as `b2`") as follows:
```rust
impl PartialOrd for AbstractByte {
    fn le(self, other: Self) -> bool {
        match (self, other) {
            /// `Uninit <= _`: initializing something makes it "more defined".
            (Uninit, _) =>
                true,
            /// Among initialized bytes, adding provenance makes it "more defined".
            (Init(data1, None), Init(data2, _)) =>
                data1 == data2,
            /// If both bytes have provenance, everything must be equal.
            (Init(data1, Some(provenance1)), Init(data2, Some(provenance2))) =>
                data1 == data2 && provenance1 == provenance2,
            /// Nothing else is related.
            _ => false,
        }
    }
}
```
Note that with `eq` already being defined (all our types that we want to compare derive `PartialEq` and `Eq`), defining `le` is sufficient to also define all the other parts of `PartialOrd`.

Similarly, on `Pointer` we say that adding provenance makes it more defined:
```rust
impl PartialOrd for Pointer {
    fn le(self, other: Self) -> bool {
        self.addr == other.addr &&
            match (self.provenance, other.provenance) {
                (None, _) => true,
                (Some(prov1), Some(prov2)) => prov1 == prov2,
                _ => false,
            }
    }
}
```

For `PointerRepr`, we say one is more defined than the other iff they have the same metadata, and one's pointer is more defined.
```rust
impl PartialOrd for PointerRepr {
    fn le(self, other: Self) -> bool {
        self.ptr <= other.ptr && self.meta == other.meta
    }
}
```

The order on `List<AbstractByte>` is assumed to be such that `bytes1 <= bytes2` if and only if they have the same length and are bytewise related by `<=`.
In fact, we define this to be in general how lists are partially ordered (based on the order of their element type):
```rust
impl<T: PartialOrd> PartialOrd for List<T> {
    fn le(self, other: Self) -> bool {
        self.len() == other.len() &&
            self.iter().zip(other.iter()).all(|(l, r)| l <= r)
    }
}
```

For `Value`, we lift the order on byte lists to relate `Bytes`s, and otherwise require equality:
```rust
impl PartialOrd for Value {
    fn le(self, other: Self) -> bool {
        match (self, other) {
            (Int(i1), Int(i2)) =>
                i1 == i2,
            (Bool(b1), Bool(b2)) =>
                b1 == b2,
            (Ptr(p1), Ptr(p2)) =>
                p1 <= p2,
            (Tuple(vals1), Tuple(vals2)) =>
                vals1 <= vals2,
            (Variant { idx: idx1, data: data1 }, Variant { idx: idx2, data: data2 }) =>
                idx == idx1 && data1 <= data2,
            (Union(chunks1), Union(chunks2)) => chunks1 <= chunks2,
            _ => false
        }
    }
}
```

Finally, on `Option<Value>` we assume that `None <= _`, and `Some(v1) <= Some(v2)` if and only if `v1 <= v2`:
```rust
impl<T: PartialOrd> PartialOrd for Option<T> {
    fn le(self, other: Self) -> bool {
        match (self, other) {
            (None, _) => true,
            (Some(l), Some(r)) => l <= r,
            _ => false
        }
    }
}
```

We say that a `v: Value` is "valid" for a type if it is a possible return value of `decode` (for an arbitrary byte list).

Now we can state the laws that we require.
First of all, `encode` and `decode` must both be "monotone":
- If `val1 <= val2` (and if both values are valid for `ty`), then `ty.encode(val1) <= ty.encode(val2)`.
- If `bytes1 <= bytes2`, then `ty.decode(val1) <= ty.decode(val2)`.

More interesting are the round-trip properties:
- If `val` is valid for `ty`, then `ty.decode(ty.encode(val)) == Some(val)`.
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

```rust
trait TypedMemory: Memory {
    /// Write a value of the given type to memory.
    /// Note that it is a spec bug if `val` cannot be encoded at `ty`!
    fn typed_store(&mut self, ptr: Self::Pointer, val: Value, pty: PlaceType) -> Result {
        let bytes = pty.type.encode(val);
        self.store(ptr, bytes, pty.align)?;
    }

    /// Read a value of the given type.
    fn typed_load(&mut self, ptr: Self::Pointer, pty: PlaceType) -> Result<Value> {
        let bytes = self.load(ptr, pty.type.size()?, pty.align)?;
        match pty.type.decode(bytes) {
            Some(val) => val,
            None => throw_ub!("load at type {ty} but the data in memory violates the validity invariant"),
        }
    }

    /// Check that the given pointer is dereferenceable according to the given layout.
    fn layout_dereferenceable(&self, ptr: Self::Pointer, layout: Layout) -> Result {
        if !layout.inhabited() {
            // TODO: I don't think Miri does this check.
            throw_ub!("uninhabited types are not dereferenceable");
        }
        self.dereferenceable(ptr, layout.size?, layout.align)?;
    }
}
```

## Relation to validity invariant

One way we *could* also use the value representation (and the author thinks this is exceedingly elegant) is to define the validity invariant.
Certainly, it is the case that if a list of bytes is not related to any value for a given type `T`, then that list of bytes is *invalid* for `T` and it should be UB to produce such a list of bytes at type `T`.
We could decide that this is an "if and only if", i.e., that the validity invariant for a type is exactly "must be in the value representation":

```rust
fn bytes_valid_for_type(ty: Type, bytes: List<AbstractByte>) -> Result {
    if ty.decode(bytes).is_none() {
        throw_ub!("data violates validity invariant of type {ty}"),
    }
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

Justifying the `dereferenceable` LLVM attribute is, in this case, left to the aliasing model (e.g. [Stacked Borrows]), just like the `noalias` attribute.

## Transmutation

The representation relation also says everything there is to say about "transmutation".
By this I mean not just the `std::mem::transmute` function, but any operation that "re-interprets data from one type at another type"
(essentially a `reinterpret_cast` in C++ terms).
Transmutation means taking a value at some type, encoding it, and then decoding it *at a different type*.
More precisely:

```rust
/// Transmutes `val` from `type1` to `type2`.
fn transmute(val: Value, type1: Type, type2: Type) -> Option<Value> {
    let bytes = type1.encode(val);
    type2.decode(bytes)
}
```

This operation can, of course, fail, which means that `val` is not valid at `type2`.

[Stacked Borrows]: stacked-borrows.md
