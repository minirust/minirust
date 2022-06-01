# MiniRust Abstract Syntax

This defines the abstract syntax of MiniRust programs.
First, the general structure of programs and functions:

```rust
/// Some opaque type of function names.
/// The details of this this is represented to not matter.
type FnName;

/// A closed MiniRust program.
struct Program {
    /// Associate a function with each declared function name.
    functions: Map<FnName, Function>,
    /// The function where execution starts.
    start: FnName,
}

/// Opaque types of names for local variables and basic blocks.
type LocalName;
type BbName;

/// A MiniRust function.
struct Function {
    /// The locals of this function, and their type.
    locals: Map<LocalName, PlaceType>,
    /// A list of locals that are initially filled with the function arguments.
    args: List<LocalName>,
    /// The name of a local that holds the return value when the function returns
    ret: LocalName,

    /// Associate each basic block name with the associated block.
    blocks: Map<BbName, BasicBlock>,
    /// The basic block where execution starts.
    start: BasicBlock,
}

/// A basic block is a sequence of statements followed by a terminator.
struct BasicBlock {
    statements: List<Statement>,
    terminator: Terminator,
}
```

And finally, the statements and terminators that MiniRust programs consist of:

```rust
enum Statement {
    /// Copy value from `source` to `target`.
    Assign {
        destination: PlaceExpr,
        source: ValueExpr,
    },
    /// Ensure that `place` contains a valid value of its type (else UB).
    Finalize {
        place: PlaceExpr,
    },
    /// Allocate the backing store for this local.
    StorageLive(LocalName),
    /// Deallocate the backing store for this local.
    StorageDead(LocalName),
}

enum Terminator {
    /// Just jump to the next block.
    Goto(BasicBlock),
    /// `condition` must evaluate to a `Value::Bool`.
    /// If it is `true`, jump to `then_block`; else jump to `else_block`.
    If {
        condition: ValueExpr,
        then_block: BbName,
        else_block: BbName,
    },
    /// Call the given function with the given arguments.
    Call {
        callee: FnName,
        /// The arguments to pass.
        arguments: List<ValueExpr>,
        /// The place to put the return value into.
        return_place: PlaceExpr,
        /// The block to jump to when this call returns.
        next_block: BbName,
    }
    /// Return from the current function.
    Return,
}

/// A "value expression" evaluates to a `Value`.
enum ValueExpr {
    /// Just return a constant.
    Constant(Value, Type),
    /// Load a value from memory.
    Load {
        /// Whether this load de-initializes the source it is loaded from ("move").
        destructive: bool,
        /// The place to load from.
        source: PlaceExpr,
    },
    /// Create a reference to a place.
    Ref {
        /// The place to create a reference to.
        target: PlaceExpr,
        /// The desired alignment of the pointee.
        align: Align,
        /// Mutability of the reference.
        mutbl: Mutability,
    },
    /// Create a raw pointer to a place.
    AddrOf {
        /// The place to create a raw pointer to.
        target: PlaceExpr,
        /// Mutability of the raw pointer.
        mutbl: Mutability,
    },
    /// Unary operators.
    UnOp {
        operator: UnOp,
        operand: ValueExpr,
    }
    /// Binary operators.
    BinOp {
        operator: BinOp,
        left: ValueExpr,
        right: ValueExpr,
    }
}

enum UnOpInt {
    /// Negate an integer value.
    Neg,
    /// Cast an integer to another.
    Cast,
}
enum UnOp {
    /// An operation on integers, with the given output type.
    Int(UnOpInt, IntType),
}

enum BinOpInt {
    /// Add two integer values.
    Add,
    /// Subtract two integer values.
    Sub,
}
enum BinOp {
    /// An operation on integers, with the given output type.
    Int(BinOpInt, IntType),
    /// Pointer arithmetic (with or without inbounds requirement).
    PtrOffset { inbounds: bool },
}

/// A "place expression" evaluates to a `Place`.
enum PlaceExpr {
    /// Denotes a local variable.
    Local(LocalName),
    /// Dereference a value (of pointer/reference type).
    Deref {
        operand: ValueExpr,
        // The alignment guarantee of the newly created place.
        align: Align,
    },
    /// Project to a field.
    Field {
        /// The place to base the projection on.
        root: PlaceExpr,
        /// The field to project to.
        field: BigInt,
    },
    /// Index to an array element.
    Index {
        /// The array to index into.
        root: PlaceExpr,
        /// The index to project to.
        index: ValueExpr,
    },
}
```

Obviously, these are all quite incomplete still.
