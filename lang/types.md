# MiniRust Types

In this document we define the set of "types" supported by MiniRust.
Note that MiniRust types play a somewhat different role than Rust types:
every Rust type corresponds to a MiniRust type, but MiniRust types are merely annotated at various operations to define how data is represented in memory.
Basically, they only define a (de)serialization format -- the **representation relation**.
In particular, MiniRust is by design *not type-safe*.
However, the representation relation is a key part of the language, since it forms the interface between the low-level and high-level view of data, between lists if (abstract) bytes and [values](values.md).
For pointer types (references and raw pointers), we types also contain a "mutability", which does not affect the representation relation but can be relevant for the aliasing rules.
(We might want to organized this differently in the future, and remove mutability from types.)

## Types

MiniRust has the following types.
Note that for now, we make the exact offsets of each field part of the type.
As always, this definition is incomplete.
In the future, we might want to separate a type from its layout, and consider these separate components -- we will have to see what works best.

```rust
enum Type {
    Int(IntType),
    Bool,
    Ref {
        mutbl: Mutability,
        pointee: Type,
    },
    RawPtr {
        mutbl: Mutability,
        pointee: Type,
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

enum Signedness {
    Unsigned,
    Signed,
}
pub use Signedness::*;

enum Mutability {
    Mutable,
    Immutable,
}
pub use Mutability::*;

/// We leave the details of enum tags to the future.
/// (We might want to extend the "variants" field of `Enum` to also have a
/// discriminant for each variant. We will see.)
enum TagEncoding { /* ... */ }
```

Note that references have no lifetime, since the lifetime is irrelevant for their representation in memory!
They *do* have a mutability since that is (or will be) relevant for the memory model.

## Type properties

Each type has a size, an alignment, and it is considered uninhabited or not.

```rust
impl Type {
    fn size(self) -> Size { /* TODO */ }
    fn align(self) -> Align { /* TODO */ }
    fn uninhabited(self) -> bool { /* TODO */ }
}
```

## Representation relation

The main purpose of types is to define how [values](values.md) are (de)serialized from memory:

```rust
impl Type {
    /// Decode a list of bytes into a value. This can fail, which typically means Undefined Behavior.
    /// `decode` must satisfy the following properties:
    ///  - `type.decode(bytes) = Some(_) -> bytes.len() == type.size()`.
    ///    In other words, all valid low-level representations must have the length given by the size of the type.
    ///  - `type.uninhabited() -> type.decode(bytes) = None`.
    ///    In other words, uninhabited type can never successfully decode anything.
    fn decode(self, bytes: List<AbstractByte>) -> Option<Value> {
        /* see below */
    }

    fn encode(self, v: Value) -> List<AbstractByte> {
        // Non-deterministically pick a list of bytes that decodes to the given value.
        pick(|bytes| self.decode(bytes) == Some(v))
    }
}
```

The definition of `decode` is huge, so we split it by type.
(We basically pretend we can have fallible patterns for the `self` parameter and declare the function multiple times with non-overlapping patterns.
If any pattern is not covered, that is a bug in the spec.)

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

### Raw pointers

For simplicity, we assume `PTR_SIZE` is 8 bytes.
TODO: Write this in a way that is generic over `PTR_SIZE`.

```rust
fn decode_ptr(bytes: List<AbstractByte>) -> Option<Pointer> {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = *bytes else { return None };
    // Get the address. Will fail if any byte is uninitialized.
    let addr = ENDIANESS.decode(signed, [b0.data()?, b1.data()?, b2.data()?, b3.data()?, b4.data()?, b5.data()?, b6.data()?, b7.data()?]).to_u64();
    // Get the provenance. Must be the same for all bytes.
    let provenance = b0.provenance();
    for b in [b0, b1, b2, b3, b4, b5, b6, b7] {
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
This avoids having two encodings of the same abstract value.

### References

```
impl Type {
    fn decode(Ref { pointee, .. }: Self, bytes: List<AbstractByte>) -> Option<Value> {
        let ptr = decode_ptr(bytes)?;
        // References need to be non-null and aligned.
        if ptr.addr == 0 { return None; }
        if ptr.addr % pointee.align() != 0 { return None; }
        // References to uninhabited types are invalid.
        // (Think: uninhabited types have impossible alignment.)
        if pointee.uninhabited() { return None; }
        Some(Value::Ptr(ptr))
    }
}
```

Note how types like `&!` are uninhabited: when the pointee type is uninhabited, there exists no valid reference to that type.

## Typed memory accesses

One key use of the value representation is to define a "typed" interface to memory.
This interface is inspired by [Cerberus](https://www.cl.cam.ac.uk/~pes20/cerberus/).

```rust
trait TypedMemory: Memory {
    /// Write a value of the given type to memory.
    fn typed_write(&mut self, ptr: Self::Pointer, val: Value, ty: Type) -> Result {
        let bytes = ty.encode(val);
        self.write(ptr, bytes)
    }

    /// Read a value of the given type.
    fn typed_read(&mut self, ptr: Self::Pointer, ty: Type) -> Result<Value> {
        let bytes = self.read(ptr, ty.size());
        Ok(ty.decode(bytes)?)
    }
}
```

## Relation to validity invariant

One way we *could* also use the value representation (and the author thinks this is exceedingly elegant) is to define the validity invariant.
Certainly, it is the case that if a list of bytes is not related to any value for a given type `T`, then that list of bytes is *invalid* for `T` and it should be UB to produce such a list of bytes at type `T`.
We could decide that this is an "if and only if", i.e., that the validity invariant for a type is exactly "must be in the value representation":

```rust
fn bytes_valid_for_type(ty: Type, bytes: List<AbstractByte>) -> Result {
  ty.decode(bytes)?;
}
```

For many types this is likely what we will do anyway (e.g., for `bool` and `!` and `()` and integers), but for references, this choice would mean that *validity of the reference cannot depend on what memory looks like*---so "dereferencable" and "points to valid data" cannot be part of the validity invariant for references.
The reason this is so elegant is that, as we have seen above, a "typed copy" already very naturally is UB when the memory that is copied is not a valid representation of `T`.
This means we do not even need a special clause in our specification for the validity invariant---in fact, the term does not even have to appear in the specification---as everything juts falls out of how a "typed copy" applies the value representation twice.

Justifying the `dereferencable` LLVM attribute is, in this case, left to the aliasing model (e.g. [Stacked Borrows]), just like the `noalias` attribute.

[Stacked Borrows]: stacked-borrows.md
