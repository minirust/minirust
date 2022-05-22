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
    /// A list of names used to refer to the function arguments.
    args: List<LocalName>,
    /// Further local variables declared inside this function.
    locals: Set<LocalName>,

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
    },
    /// Unary operators.
    UnOp {
        op: BinOp,
        operand: ValueExpr,
    }
    /// Binary operators.
    BinOp {
        left: ValueExpr,
        op: BinOp,
        right: ValueExpr,
    }
}

enum UnOp {
    /// Negate an integer value.
    INeg(IntType),
}

enum BinOp {
    /// Add two integer values.
    IAdd(IntType),
}

/// A "place expression" evaluates to a `Place`.
enum PlaceExpr {
    /// Denotes a local variable.
    Local(LocalName),
    /// Dereference a value (of pointer/reference type).
    Deref(ValueExpr),
}

/// For now, a `Place` is just a pointer.
/// (But this might have to change.)
type Place = Pointer;
```

Obviously, these are all quite incomplete still.

## Well-formed programs

MiniRust programs need to satisfy some conditions to be well-formed, e.g. all `PlaceExpr::Local` need to refer to a local that actually exists in the current function.

TODO: define this.
