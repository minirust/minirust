# MiniRust Abstract Syntax

This defines the abstract syntax of MiniRust programs.

## Programs and functions

First, the general structure of programs and functions:

```rust
/// Opaque types of names for functions and globals.
/// The internal representations of these types do not matter.
pub struct FnName(pub libspecr::Name);
pub struct GlobalName(pub libspecr::Name);

/// A closed MiniRust program.
pub struct Program {
    /// Associate a function with each declared function name.
    pub functions: Map<FnName, Function>,
    /// The function where execution starts.
    pub start: FnName,
    /// Associate each global name with the associated global.
    pub globals: Map<GlobalName, Global>,
}

/// Opaque types of names for local variables and basic blocks.
pub struct LocalName(pub libspecr::Name);
pub struct BbName(pub libspecr::Name);

/// A MiniRust function.
pub struct Function {
    /// The locals of this function, and their type.
    pub locals: Map<LocalName, PlaceType>,
    /// A list of locals that are initially filled with the function arguments.
    /// Also determines the call ABI for each argument.
    pub args: List<(LocalName, ArgAbi)>,
    /// The name of a local that holds the return value when the function returns.
    /// Can be `None` if this function will not return.
    /// Also determines the return ABI.
    pub ret: Option<(LocalName, ArgAbi)>,

    /// Associate each basic block name with the associated block.
    pub blocks: Map<BbName, BasicBlock>,
    /// The basic block where execution starts.
    pub start: BbName,
}

/// A basic block is a sequence of statements followed by a terminator.
pub struct BasicBlock {
    pub statements: List<Statement>,
    pub terminator: Terminator,
}
```

## Statements, terminators, expressions

Next, the statements and terminators that MiniRust programs consist of:

```rust
pub enum Statement {
    /// Copy value from `source` to `destination`.
    Assign {
        destination: PlaceExpr,
        source: ValueExpr,
    },
    /// Ensure that `place` contains a valid value of its type (else UB).
    /// Also perform retagging.
    Finalize {
        place: PlaceExpr,
        /// Indicates whether this operation occurs as part of the prelude
        /// that we have at the top of each function (which affects retagging).
        fn_entry: bool,
    },
    /// Allocate the backing store for this local.
    StorageLive(LocalName),
    /// Deallocate the backing store for this local.
    StorageDead(LocalName),
}

pub enum Terminator {
    /// Just jump to the next block.
    Goto(BbName),
    /// `condition` must evaluate to a `Value::Bool`.
    /// If it is `true`, jump to `then_block`; else jump to `else_block`.
    If {
        condition: ValueExpr,
        then_block: BbName,
        else_block: BbName,
    },
    /// If this is ever executed, we have UB.
    Unreachable,
    /// Call the given function with the given arguments.
    Call {
        callee: ValueExpr,
        /// The arguments to pass, and which ABIs to use for that.
        arguments: List<(ValueExpr, ArgAbi)>,
        /// The place to put the return value into, and which ABI to use for that.
        /// If `None`, the function's return value will be discarded.
        ret: Option<(PlaceExpr, ArgAbi)>,
        /// The block to jump to when this call returns.
        /// If `None`, UB will be raised when the function returns.
        next_block: Option<BbName>,
    },
    /// Call the given intrinsic with the given arguments.
    CallIntrinsic {
        intrinsic: Intrinsic,
        /// The arguments to pass.
        arguments: List<ValueExpr>,
        /// The place to put the return value into.
        /// If `None`, the intrinsic's return value will be discarded.
        ret: Option<PlaceExpr>,
        /// The block to jump to when this call returns.
        /// If `None`, UB will be raised when the intrinsic returns.
        next_block: Option<BbName>,
    },
    /// Return from the current function.
    Return,
}

pub enum LockIntrinsic {
    Acquire,
    Release,
    Create,
}

pub enum Intrinsic {
    Exit,
    PrintStdout,
    PrintStderr,
    Allocate,
    Deallocate,
    Spawn,
    Join,
    Lock(LockIntrinsic),
}
```

We also need to define constants (a strict subset of `Value`).

```rust
/// Constants are Values, but cannot have provenance.
/// Currently we do not support Ptr and Union constants.
pub enum Constant {
    /// A mathematical integer, used for `i*`/`u*` types.
    Int(Int),
    /// A Boolean value, used for `bool`.
    Bool(bool),
    /// A pointer pointing into a global allocation with a given offset.
    GlobalPointer(Relocation),
    /// A pointer pointing to a function.
    FnPointer(FnName),

    /// A variant of a sum type, used for enums.
    // TODO Variant shouldn't be a Constant, but rather a ValueExpr.
    Variant {
        idx: Int,
        #[specr::indirection]
        data: Constant,
    },
}
```

And finally, the syntax of expressions:

```rust
/// A "value expression" evaluates to a `Value`.
pub enum ValueExpr {
    /// Just return a constant value.
    Constant(Constant, Type),

    /// An n-tuple, used for arrays, structs, tuples (including unit).
    Tuple(List<ValueExpr>, Type),

    /// A `Union` value.
    Union {
        /// The union's field which will be initialized.
        field: Int,
        /// The value it will be initialized with.
        #[specr::indirection]
        expr: ValueExpr,
        /// The union type, needs to be `Type::Union`
        union_ty: Type,
    },

    /// Load a value from memory.
    Load {
        /// Whether this load de-initializes the source it is loaded from ("move").
        destructive: bool,
        /// The place to load from.
        #[specr::indirection]
        source: PlaceExpr,
    },
    /// Create a pointer to a place.
    AddrOf {
        /// The place to create a pointer to.
        #[specr::indirection]
        target: PlaceExpr,
        /// The type of the created pointer.
        ptr_ty: PtrType,
    },
    /// Unary operators.
    UnOp {
        operator: UnOp,
        #[specr::indirection]
        operand: ValueExpr,
    },
    /// Binary operators.
    BinOp {
        operator: BinOp,
        #[specr::indirection]
        left: ValueExpr,
        #[specr::indirection]
        right: ValueExpr,
    },
}

pub enum UnOpInt {
    /// Negate an integer value.
    Neg,
    /// Cast an integer to another.
    Cast,
}
pub enum UnOp {
    /// An operation on integers, with the given output type.
    Int(UnOpInt, IntType),
    /// Pointer-to-pointer cast
    Ptr2Ptr(PtrType),
    /// Pointer-to-integer cast
    Ptr2Int,
    /// Integer-to-pointer cast
    Int2Ptr(PtrType),
}

pub enum BinOpInt {
    /// Add two integer values.
    Add,
    /// Subtract two integer values.
    Sub,
    /// Multiply two integer values.
    Mul,
    /// Divide two integer values.
    /// Division by zero is UB.
    Div,
    /// Remainder of a division, the `%` operator.
    /// Throws UB, if the modulus (second operand) is zero.
    Rem,
}

/// A relation between integers.
pub enum IntRel {
    /// less than
    Lt,
    /// greater than
    Gt,
    /// less than or equal
    Le,
    /// greater than or equal
    Ge,
    /// Equality
    Eq,
    /// Inequality
    Ne,
}

pub enum BinOp {
    /// An operation on integers, with the given output type.
    Int(BinOpInt, IntType),
    /// A relation between integers.
    IntRel(IntRel),
    /// Pointer arithmetic (with or without inbounds requirement).
    PtrOffset { inbounds: bool },
}

/// A "place expression" evaluates to a `Place`.
pub enum PlaceExpr {
    /// Denotes a local variable.
    Local(LocalName),
    /// Dereference a value (of pointer/reference type).
    Deref {
        #[specr::indirection]
        operand: ValueExpr,
        // The type of the newly created place.
        ptype: PlaceType,
    },
    /// Project to a field.
    Field {
        /// The place to base the projection on.
        #[specr::indirection]
        root: PlaceExpr,
        /// The field to project to.
        field: Int,
    },
    /// Index to an array element.
    Index {
        /// The array to index into.
        #[specr::indirection]
        root: PlaceExpr,
        /// The index to project to.
        #[specr::indirection]
        index: ValueExpr,
    },
}

/// A global allocation.
pub struct Global {
    /// The raw bytes of the allocation. `None` represents uninitialized bytes.
    pub bytes: List<Option<u8>>,
    /// Cross-references pointing to other global allocations,
    /// together with an offset, expressing where this allocation should put the pointer.
    /// Note that the pointers created due to relocations overwrite the data given by `bytes`.
    pub relocations: List<(Size, Relocation)>,
    /// The align with which this global shall be allocated.
    pub align: Align,
}

/// A pointer into a global allocation.
pub struct Relocation {
    /// The name of the global allocation we are pointing into.
    pub name: GlobalName,
    /// The offset within that allocation.
    pub offset: Size,
}

```

Obviously, these are all quite incomplete still.
