# MiniRust Values and Types

The purpose of this file is to describe what the set of *all possible values* is in MiniRust, and how they are represented in memory.
This is one of the key definitions in MiniRust.
The representation relation relates values with lists of [abstract bytes](../mem/interface.md#abstract-bytes):
it defines, for a given value and list of bytes, whether that value is represented by that list.
However, before we can even start specifying the relation, we have to specify the domains of abstract bytes (part of the [memory interface](../mem/interface.md)) and of values (this file).

[representation]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/reference/src/glossary.md#representation
[memory-interface]: memory-interface.md

## Values

The MiniRust value domain is described by the following type definition.

```rust
#[derive(PartialEq, Eq)]
enum Value {
    /// A mathematical integer, used for `i*`/`u*` types.
    Int(BigInt),
    /// A Boolean value, used for `bool`.
    Bool(bool),
    /// A pointer value, used for (thin) references and raw pointers.
    Ptr(Pointer),
    /// An n-tuple, used for arrays, structs, tuples (including unit).
    Tuple(List<Value>),
    /// A variant of a sum type, used for enums.
    Variant {
        idx: BigInt,
        data: Value,
    },
    /// A "bag of bytes", used for unions.
    Bytes(List<AbstractByte>),
}
```

The point of this type is to capture the mathematical concepts that are represented by the data we store in memory.
The definition is likely incomplete, and even if it was complete now, we might expand it as Rust grows.
That is okay; all previously defined representation relations are still well-defined when the domain grows, the newly added values will just not be valid for old types as one would expect.

## Types

Note that MiniRust types play a somewhat different role than Rust types:
every Rust type corresponds to a MiniRust type, but MiniRust types are merely annotated at various operations to define how data is represented in memory.
Basically, they only define a (de)serialization format -- the **representation relation**, define by an "encode" function to turn values into byte lists, and a "decode" function for the opposite operation.
In particular, MiniRust is by design *not type-safe*.
However, the representation relation is a key part of the language, since it forms the interface between the low-level and high-level view of data, between lists if (abstract) bytes and [values](values.md).
For pointer types (references and raw pointers), we types also contain a "mutability", which does not affect the representation relation but can be relevant for the aliasing rules.
(We might want to organize this differently in the future, and remove mutability from types.)

MiniRust has the following types.
Note that for now, we make the exact offsets of each field part of the type.
As always, this definition is incomplete.
In the future, we might want to separate a type from its layout, and consider these separate components -- we will have to see what works best.

```rust
/// The "layout" of a type defines its outline or shape.
struct Layout {
    size: Size,
    align: Align,
    inhabited: bool,
}

enum Type {
    Int(IntType),
    Bool,
    Ref {
        mutbl: Mutability,
        /// We only need to know the layout of the pointee.
        /// (This also means we have a finite representation even when the Rust type is recursive.)
        pointee: Layout,
    },
    Box {
        pointee: Layout,
    },
    RawPtr {
        mutbl: Mutability,
        /// TODO: do we need this at all?
        pointee: Layout,
    },
    /// "Tuple" is used for all heterogeneous types, i.e., both Rust tuples and structs.
    /// It is also used for arrays; then all fields will have the same type.
    Tuple {
        /// Fields must not overlap.
        fields: Fields,
        /// The total size of the type can indicate trailing padding.
        /// Must be large enough to contain all fields.
        size: Size,
    },
    Enum {
        /// Each variant is given by a list of fields.
        /// The "variant index" of a variant is its index in this list.
        /// (The Rust type `!` is encoded as an `Enum` with an empty list of variants.)
        variants: List<Fields>,
        /// This contains all the tricky details of how to encode the active variant
        /// at runtime.
        tag_encoding: TagEncoding,
        /// The total size of the type can indicate trailing padding.
        /// Must be large enough to contain all fields of all variants.
        size: Size,
    },
    Union {
        /// Fields *may* overlap.
        fields: Fields,
        /// The total size of the type can indicate trailing padding.
        /// Must be large enough to contain all fields.
        size: Size,
    },
}

struct IntType {
    signed: Signedness,
    size: Size,
}

type Fields = List<(Size, Type)>; // (offset, type) pair for each field

/// We leave the details of enum tags to the future.
/// (We might want to extend the "variants" field of `Enum` to also have a
/// discriminant for each variant. We will see.)
enum TagEncoding { /* ... */ }
```

Note that references have no lifetime, since the lifetime is irrelevant for their representation in memory!
They *do* have a mutability since that is (or will be) relevant for the memory model.

### Well-formed types

Not all types are well-formed; for example, the fields of a `Tuple` must not overlap.

- TODO: define this

## Type properties

Each type has a layout.

- TODO: define this

```rust
impl Type {
    fn layout(self) -> Layout;

    fn size(self) -> Size { self.layout().size }
    fn align(self) -> Align { self.layout().align }
    fn inhabited(self) -> bool { self.layout().inhabited }
}
```

## Representation relation

The main purpose of types is to define how [values](values.md) are (de)serialized from memory.
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
            [AbstractByte::Init(0, _)] => Some(Value::Bool(false)),
            [AbstractByte::Init(1, _)] => Some(Value::Bool(true)),
            _ => None,
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
        let [AbstractByte::Init(b0, _), AbstractByte::Init(b1, _)] = *bytes else { return None };
        Some(Value::Int(ENDIANESS.decode(signed, [b0, b1])))
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
- TODO: This definition says that when multiple provenances are mixed, the pointer has `None` provenance, i.e., it is "invalid".
  Is that the semantics we want? Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/286#issuecomment-1136948796).
- TODO: This does not allow uninitialized integers. I think that is fairly clearly what we want, also considering LLVM is moving towards using `noundef` heavily to avoid many of the current issues in their `undef` handling. But this is also still [being discussed](https://github.com/rust-lang/unsafe-code-guidelines/issues/71).

### Raw pointers

Decoding pointers is a bit inconvenient since we do not know `PTR_SIZE`.

```rust
fn decode_ptr(bytes: List<AbstractByte>) -> Option<Pointer> {
    if bytes.len() != PTR_SIZE { return None; }
    // Convert into list of bytes; fail if any byte is uninitialized.
    let bytes_data: [u8; PTR_SIZE] = bytes.map(|b| b.data()).collect()?;
    let addr = ENDIANESS.decode(Unsigned, &bytes_data).to_u64();
    // Get the provenance. Must be the same for all bytes, else we use `None`.
    let mut provenance: Option<Provenance> = bytes[0].provenance();
    for b in bytes {
        if b.provenance() != provenance {
            provenance = None;
        }
    }
    Some(Pointer { addr, provenance })
}

fn encode_ptr(ptr: Pointer) -> List<AbstractByte> {
    let bytes_data: [u8; PTR_SIZE] = ENDIANESS.encode(Unsigned, ptr.addr).unwrap();
    bytes_data
        .map(|b| AbstractByte::Init(b, ptr.provenance))
        .collect()
}

impl Type {
    fn decode(RawPtr { .. }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        Some(Value::Ptr(decode_ptr(bytes)?))
    }
    fn encode(RawPtr { .. }: Self, val: Value) -> List<AbstractByte> {
        let Value::Ptr(ptr) = val else { panic!() };
        encode_ptr(ptr)
    }
}
```

### References and `Box`

```rust
/// Check if the given pointer is valid for safe pointer types (`Ref`, `Box`).
fn check_safe_ptr(ptr: Pointer, pointee: Layout) -> bool {
    // References (and `Box`) need to be non-null, aligned, and not point to an uninhabited type.
    // (Think: uninhabited types have impossible alignment.)
    ptr.addr != 0 && ptr.addr % pointee.align == 0 && pointee.inhabited
}

impl Type {
    fn decode(Ref { pointee, .. } | Box { pointee }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        let ptr = decode_ptr(bytes)?;
        if !check_safe_ptr(ptr, pointee) { return None; }
        Some(Value::Ptr(ptr))
    }
    fn encode(Ref { .. } | Box { .. }: Self, val: Value) -> List<AbstractByte> {
        let Value::Ptr(ptr) = val else { panic!() };
        encode_ptr(ptr)
    }
}
```

Note that types like `&!` have no valid value: when the pointee type is uninhabited (in the sense of `!ty.inhabited()`), there exists no valid reference to that type.
This means we could make `&!` itself have `inhabited: false` in its layout; I consider that decision part of "compiling surface Rust to MiniRust" and thus out-of-scope for this document.

- TODO: Do we really want to special case references to uninhabited types? Do we somehow want to require more, like pointing to a valid instance of the pointee type?
  (The latter would not even be possible with the current structure of MiniRust.)
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/77).

### Tuples (and arrays, structs, ...)

For simplicity, we only define pairs for now.

```rust
impl Type {
    fn decode(Tuple { fields: [field1, field2], size }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        if bytes.len() != size { return None; }
        let (size1, type1) = field1;
        let val1 = type1.decode(bytes[size1..][..type1.size()]);
        let (size2, type2) = field2;
        let val2 = type1.decode(bytes[size2..][..type2.size()]);
        Some(Value::Tuple([val1, val2]))
    }
    fn encode(Tuple { fields: [field1, field2], size }: Self, val: Value) -> List<AbstractByte> {
        let Value::Tuple([val1, val2]) = val else { panic!() };
        let mut bytes = [AbstractByte::Uninit; size];
        let (size1, type1) = field1;
        bytes[size1..][..type1.size()] = type1.encode(val1);
        let (size2, type2) = field2;
        bytes[size2..][..type2.size()] = type2.encode(val2);
        bytes
    }
}
```

Note in particular that `decode` ignores the bytes which are before, between, or after the fields (usually called "padding").
`encode` in turn always and deterministically makes those bytes `Uninit`.
(The [generic properties](#generic-properties) defined below make this the only possible choice for `encode`.)

### Unions

A union simply stores the bytes directly, no high-level interpretation of data happens.

- TODO: Some real unions actually do not preserve all bytes, they [can have padding](https://github.com/rust-lang/unsafe-code-guidelines/issues/156).
  So we have to model that there can be "gaps" between the parts of the byte list that are preserved perfectly.
- TODO: Should we require *some* kind of validity? See [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/73).

```rust
impl Type {
    fn decode(Union { size, .. }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        if bytes.len() != size { return None; }
        Some(Value::Bytes(bytes))
    }
    fn encode(Union { size, .. }: Self, value: Value) -> List<AbstractByte> {
        let Value::Bytes(bytes) = val else { panic!() };
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
            (Init(data1, Some(provenance1)), Init(data2, Some(provenance2)) =>
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
                idx == idx1 && data1 <= data2
            (Bytes(bytes1), Bytes(bytes2)) => bytes1 <= bytes2,
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
    fn typed_store(&mut self, ptr: Self::Pointer, val: Value, ty: Type) -> Result {
        let bytes = ty.encode(val);
        self.store(ptr, bytes)
    }

    /// Read a value of the given type.
    fn typed_load(&mut self, ptr: Self::Pointer, ty: Type) -> Result<Value> {
        let bytes = self.load(ptr, ty.size());
        match ty.decode(bytes) {
            Some(val) => Ok(val),
            None => throw_ub!("load at type {ty} but the data in memory violates the validity invariant"),
        }
    }

    /// Check that the given pointer is dereferenceable according to the given layout.
    fn layout_dereferenceable(&self, ptr: Self::Pointer, layout: Layout) -> Result {
        if !layout.inhabited() {
            // TODO: I don't think Miri does this check.
            throw_ub!("uninhabited types are not dereferenceable");
        }
        self.dereferenceable(ptr, layout.size, layout.align)?;
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

[Stacked Borrows]: stacked-borrows.md
