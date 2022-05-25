# MiniRust Values and Types

The purpose of this file is to describe what the set of *all possible values* is in MiniRust, and how they are represented in memory.
This is one of the key definitions in MiniRust.
The representation relation relates values with lists of [abstract bytes](../mem/interface.md#abstract-bytes):
it defines, for a given value and list of bytes, whether that value is represented by that list.
However, before we can even start specifying the relation, we have to specify the domains of abstract bytes (part of the [memory interface](../mem/interface.md) and of values (this document).

[representation]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/reference/src/glossary.md#representation
[memory-interface]: memory-interface.md

## Values

The MiniRust value domain is described by the following type definition.

```rust
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
    /// A "bag of raw bytes", used for unions.
    RawBag(List<AbstractByte>),
}
```

The point of this type is to capture the mathematical concepts that are represented by the data we store in memory.
The definition is likely incomplete, and even if it was complete now, we might expand it as Rust grows.
That is okay; all previously defined representation relations are still well-defined when the domain grows, the newly added values will just not be valid for old types as one would expect.

## Types

Note that MiniRust types play a somewhat different role than Rust types:
every Rust type corresponds to a MiniRust type, but MiniRust types are merely annotated at various operations to define how data is represented in memory.
Basically, they only define a (de)serialization format -- the **representation relation**.
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
`decode` converts a list of bytes into a value; this operation can fail but it is otherwise functional (i.e., a given list of bytes encodes at most one value).
`encode` inverts `decode`; we can define it generically by saying "just pick a list of bytes that would decode to the given value".
This means that starting with a `value`, if we encode and then decode it, we will definitely get that `value` back (assuming it is valid for the given type),
and if we start with some `bytes`, if we decode and then encode them, we have a set of possible encodings to pick from but that set definitely contains the original `bytes` that we started with (again assuming it is valid for the given type).
This is called an "adjoint" relationship, or a "Galois connection".

The definition of `decode` is huge, so we split it by type.
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
    fn encode(self, v: Value) -> List<AbstractByte> {
        // Non-deterministically pick a list of bytes that decodes to the given value.
        pick(|bytes| self.decode(bytes) == Some(v))
    }
}
```

- TODO: Define this for the other types.

### `bool`

```rust
impl Type {
    fn decode(Bool: Self, bytes: List<AbstractByte>) -> Option<Value> {
        match *bytes {
            [AbstractByte::Raw(0)] => Some(Value::Bool(false)),
            [AbstractByte::Raw(1)] => Some(Value::Bool(true)),
            _ => None,
        }
    }
}
```

Note, in particular, that an `AbstractByte::Ptr` is *not* valid for `bool!`
This corresponds to ruling out ptr-to-bool transmutation.

### Integers

For now we only define `u16` and `i16`.

```rust
impl Type {
    fn decode(Int(IntType { signed, size: Size::from_bits(16) }): Self, bytes: List<AbstractByte>) -> Option<Value> {
        let [AbstractByte::Raw(b0), AbstractByte::Raw(b1)] = *bytes else { return None };
        Some(Value::Int(ENDIANESS.decode(signed, [b0, b1])))
    }
}
```

Again, if any byte is `AbstractByte::Ptr` this will return `None`.
That corresponds to ruling out ptr-to-int transmutation.

- TODO: Is that the right semantics? See [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/286).

### Raw pointers

Decoding pointers is a bit inconvenient since we do not know `PTR_SIZE`.

```rust
fn decode_ptr(bytes: List<AbstractByte>) -> Option<Pointer> {
    if bytes.len() != PTR_SIZE { return None; }
    // Convert into list of bytes; fail if any byte is uninitialized.
    let bytes_data: [u8; PTR_SIZE] = bytes.map(|b| b.data()).collect()?;
    let addr = ENDIANESS.decode(signed, &bytes_data).to_u64();
    // Get the provenance. Must be the same for all bytes.
    let provenance: Option<Provenance> = bytes[0].provenance();
    for b in bytes {
        if b.provenance() != provenance { return None; }
    }
    Some(Pointer { addr, provenance })
}

impl Type {
    fn decode(RawPtr { .. }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        Some(Value::Ptr(decode_ptr(bytes)?))
    }
}
```

Note that, crucially, a pointer with "invalid" (`None`) provenance is never encoded as `AbstractByte::Ptr`.
(This is not even structurally possible: `Pointer` carries `Option<Provenance>` while `AbstractByte::Ptr` carries `Provenance`.)
This avoids having two encodings of the same abstract value.

- TODO: This definition fails to decode a pointer unless the provenance is the same for *all* its bytes.
  Is that the semantics we want? It seems most conservative. Also, if we want to keep the perfect symmetry of decode and encode,
  if during decoding we allow some bytes to have no provenance (and then use the provenance of the remaining bytes),
  then during encoding we allow the machine to pick an arbitrary subset of bytes and *not* give it provenance.
  That seems rather odd.

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
}
```

Note that types like `&!` are uninhabited: when the pointee type is uninhabited, there exists no valid reference to that type.

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

    /// Check that the given pointer is dereferencable according to the given layout.
    fn layout_dereferencable(&self, ptr: Self::Pointer, layout: Layout) -> Result {
        if !layout.inhabited() {
            // TODO: I don't think Miri does this check.
            throw_ub!("uninhabited types are not dereferencable");
        }
        self.dereferencable(ptr, layout.size, layout.align)?;
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

For many types this is likely what we will do anyway (e.g., for `bool` and `!` and `()` and integers), but for references, this choice would mean that *validity of the reference cannot depend on what memory looks like*---so "dereferencable" and "points to valid data" cannot be part of the validity invariant for references.
The reason this is so elegant is that, as we have seen above, a "typed copy" already very naturally is UB when the memory that is copied is not a valid representation of `T`.
This means we do not even need a special clause in our specification for the validity invariant---in fact, the term does not even have to appear in the specification---as everything juts falls out of how a "typed copy" applies the value representation twice.

Justifying the `dereferencable` LLVM attribute is, in this case, left to the aliasing model (e.g. [Stacked Borrows]), just like the `noalias` attribute.

[Stacked Borrows]: stacked-borrows.md
