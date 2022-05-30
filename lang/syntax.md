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
    /// A list of names used to refer to the function arguments, and their layouts.
    /// The caller will allocate these when creating the stack frame.
    args: List<(LocalName, Layout)>,
    /// The name used to refer to the local that stores the return value.
    /// The caller will allocate this when creating the stack frame.
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
        type: Type,
        source: ValueExpr,
    },
    /// Ensure that `place` contains a valid value of type `type` (else UB).
    Finalize {
        place: PlaceExpr,
        type: Type,
    },
    /// Allocate the backing store for this local.
    StorageLive(LocalName, Type),
    /// Deallocate the backing store for this local.
    StorageDead(LocalName, Type),
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
        arguments: List<(ValueExpr, Type)>,
        return_place: PlaceExpr,
        next_block: BbName,
    }
    /// Return from the current function.
    Return,
}

/// A "value expression" evaluates to a `Value`.
enum ValueExpr {
    /// Just return a constant.
    Constant(Value),
    /// Load a value from memory.
    Load {
        /// Whether this load de-initializes the source it is loaded from ("move").
        destructive: bool,
        /// The place to load from.
        source: PlaceExpr,
        /// The type to load at.
        type: Type,
    },
    /// Take the address of ("create a reference to") a place.
    Ref {
        /// The place to create a reference to.
        target: PlaceExpr,
        /// The type of the place. Must be a reference or raw pointer type.
        type: Type,
    },
    /// Unary operators.
    UnOp {
        operator: BinOp,
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
        pointee: Layout,
    }
    /// Project to a field.
    Field {
        /// The place to base the projection on.
        root: PlaceExpr,
        /// The type of `root`.
        type: Type,
        /// The field to project to.
        field: usize,
    }
}
```

Obviously, these are all quite incomplete still.
