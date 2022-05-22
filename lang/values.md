# MiniRust Values

The purpose of this document is to describe what the set of *all possible values* is in MiniRust.
This is an important definition: one key element of a Rust specification will be to define the [representation relation][representation] of every type.
This relation relates values with lists of [abstract bytes](../mem/interface.md#abstract-bytes):
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
