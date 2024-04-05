# MiniRust Abstract Syntax

This defines the abstract syntax of MiniRust programs.

## Expressions

MiniRust has two kinds of expressions:
*value expressions* evaluate to a value and are found, in particular, on the right-hand side of assignments;
*place expressions* evaluate to a place and are found, in particular, in the left-hand side of assignments.

Obviously, these are all quite incomplete still.

### Value expressions

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

    /// A variant of an enum type.
    Variant {
        /// The discriminant of the variant.
        discriminant: Int,
        /// The `ValueExpr` for the variant.
        #[specr::indirection]
        data: ValueExpr,
        /// The enum type, needs to be `Type::Enum`.
        enum_ty: Type,
    },

    /// Read the discriminant of an enum type.
    /// As we don't need to know the validity of the inner data
    /// we don't fully load the variant value.
    GetDiscriminant {
        /// The place where the enum is located.
        #[specr::indirection]
        place: PlaceExpr,
    },

    /// Load a value from memory.
    Load {
        /// The place to load from.
        #[specr::indirection]
        source: PlaceExpr,
    },
    /// Create a pointer (raw pointer or reference) to a place.
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

/// Constants are basically values, but cannot have provenance.
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
    /// A pointer with constant address, not pointing into any allocation.
    PointerWithoutProvenance(Address),
}

pub enum UnOpInt {
    /// Negate an integer value arithmetically (`x` becomes `-x`).
    Neg,
}
pub enum UnOpBool {
    /// Boolean negation.
    Not,
}
pub enum CastOp {
    /// Argument can be any integer type; returns the given integer type.
    IntToInt(IntType),
    /// Argument is a Boolean; returns the given integer type.
    /// True becomes `Int::ONE` and false `Int::ZERO`.
    BoolToInt(IntType),
    /// Transmute the value to a different type.
    /// The program is well-formed even if the output type has a different size than the
    /// input type, but the operation is UB in that case.
    Transmute(Type),
}
pub enum UnOp {
    /// An operation on an integer; returns an integer of the same type.
    Int(UnOpInt),
    /// An operation on a boolean; returns a boolean.
    Bool(UnOpBool),
    /// A form of cast.
    Cast(CastOp),
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
    /// Bitwise-and two integer values.
    BitAnd
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
    /// equal
    Eq,
    /// inequal
    Ne,
}
pub enum BinOpBool {
    /// Bitwise-and on booleans.
    BitAnd,
} 
pub enum BinOp {
    /// An operation on integers (both must have the same type); returns an integer of the same type.
    Int(BinOpInt),
    /// A relation between integers (both must have the same type); returns a boolean.
    IntRel(IntRel),
    /// Pointer arithmetic (with or without inbounds requirement);
    /// takes a pointer as left operand and an integer as right operand;
    /// returns a pointer.
    PtrOffset { inbounds: bool },
    /// An operation on booleans
    Bool(BinOpBool),
}
```

### Place expressions

```rust
/// A "place expression" evaluates to a `Place`.
pub enum PlaceExpr {
    /// Denotes a local variable.
    Local(LocalName),
    /// Dereference a value (of pointer/reference type).
    Deref {
        #[specr::indirection]
        operand: ValueExpr,
        // The type of the newly created place.
        ty: Type,
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
    /// Enum variant downcast.
    Downcast {
        /// The base enum to project to the specific variant.
        #[specr::indirection]
        root: PlaceExpr,
        /// The discriminant of the variant to project to.
        discriminant: Int,
    },
}
```

## Statements, terminators

Next, the statements and terminators that MiniRust programs consist of:

```rust
pub enum Statement {
    /// Copy value from `source` to `destination`.
    Assign {
        destination: PlaceExpr,
        source: ValueExpr,
    },
    /// Set the discriminant of the variant at `destination` to `value`.
    SetDiscriminant {
        destination: PlaceExpr,
        value: Int,
    },
    /// Ensure that `place` contains a valid value of its type (else UB).
    /// Also perform retagging and ensure safe pointers are dereferenceable.
    ///
    /// The frontend is generally expected to generate this for all function argument,
    /// and possibly in more places.
    Validate {
        place: PlaceExpr,
        /// Indicates whether this operation occurs as part of the prelude
        /// that we have at the top of each function (which affects retagging).
        fn_entry: bool,
    },
    /// De-initialize a place.
    Deinit {
        place: PlaceExpr,
    },
    /// Allocate the backing store for this local.
    StorageLive(LocalName),
    /// Deallocate the backing store for this local.
    StorageDead(LocalName),
}

pub enum Terminator {
    /// Just jump to the next block.
    Goto(BbName),
    /// `value` needs to evaluate to a `Value::Int`.
    /// `cases` map those values to blocks to jump to and therefore have to have the equivalent type.
    /// If no value matches we fall back to the block given in `fallback`.
    Switch {
        value: ValueExpr,
        cases: Map<Int, BbName>,
        fallback: BbName,
    },
    /// If this is ever executed, we have UB.
    Unreachable,
    /// Call the given function with the given arguments.
    Call {
        callee: ValueExpr,
        /// The arguments to pass.
        arguments: List<ArgumentExpr>,
        /// The place to put the return value into.
        ret: PlaceExpr,
        /// The block to jump to when this call returns.
        /// If `None`, UB will be raised when the function returns.
        next_block: Option<BbName>,
    },
    /// Call the given intrinsic function with the given arguments.
    CallIntrinsic {
        intrinsic: Intrinsic,
        /// The arguments to pass.
        arguments: List<ValueExpr>,
        /// The place to put the return value into.
        ret: PlaceExpr,
        /// The block to jump to when this call returns.
        /// If `None`, UB will be raised when the intrinsic returns.
        next_block: Option<BbName>,
    },
    /// Return from the current function.
    Return,
}

/// Function arguments can be passed by-value or in-place.
pub enum ArgumentExpr {
    /// Pass a copy of this value to the function.
    ///
    /// Technically this could be encoded by generating a fresh temporary, copying the value there, and doing in-place passing.
    /// FIXME: is it worth providing this mode anyway?
    ByValue(ValueExpr),
    /// Pass the argument value in-place; the contents of this place may be altered arbitrarily by the callee.
    InPlace(PlaceExpr),
}

pub enum LockIntrinsic {
    Acquire,
    Release,
    Create,
}

pub enum Intrinsic {
    Assume,
    Exit,
    PrintStdout,
    PrintStderr,
    Allocate,
    Deallocate,
    Spawn,
    Join,
    AtomicStore,
    AtomicLoad,
    AtomicCompareExchange,
    AtomicFetchAndOp(BinOpInt),
    Lock(LockIntrinsic),
    /// 'Expose' the provenance a pointer so that it can later be cast to an integer.
    /// The address part of the pointer is stored in `destination`.
    PointerExposeProvenance,
    /// Create a new pointer from the given address with some previously exposed provenance.
    PointerWithExposedProvenance,
}
```

## Programs and functions

Finally, the general structure of programs and functions:

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
    pub locals: Map<LocalName, Type>,
    /// A list of locals that are initially filled with the function arguments.
    pub args: List<LocalName>,
    /// The name of a local that holds the return value when the function returns.
    pub ret: LocalName,
    /// The call calling convention of this function.
    pub calling_convention: CallingConvention,

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

/// A global allocation.
pub struct Global {
    /// The raw bytes of the allocation. `None` represents uninitialized bytes.
    pub bytes: List<Option<u8>>,
    /// Cross-references pointing to other global allocations,
    /// together with an offset, expressing where this allocation should put the pointer.
    /// Note that the pointers created due to relocations overwrite the data given by `bytes`.
    pub relocations: List<(Offset, Relocation)>,
    /// The alignment with which this global shall be allocated.
    pub align: Align,
}

/// A pointer into a global allocation.
pub struct Relocation {
    /// The name of the global allocation we are pointing into.
    pub name: GlobalName,
    /// The offset within that allocation.
    pub offset: Offset,
}
```
