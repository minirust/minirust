//! This module makes it easy to create a `Program`.
//!
//! Example:
//!
//! ```rust
//! // Our main function has one local of type `usize`.
//! let locals = &[<usize>::get_ptype()];
//!
//! // the basic block `bb` allocates space for this local, and then terminates the program.
//! let bb = block!(storage_live(0), exit());
//!
//! // the function `f` is our main function, it does never return and has no function arguments.
//! let f = function(Ret::No, 0, locals, &[bb]);
//!
//! // Our program only consists of the function `f`.
//! let program = program(&[f]);
//! ```

use crate::*;

mod function;
pub use function::*;

mod global;
pub use global::*;

mod statement;
pub use statement::*;

mod terminator;
pub use terminator::*;

mod expr;
pub use expr::*;

mod ty;
pub use ty::*;

mod ty_conv;
pub use ty_conv::*;

pub struct ProgramBuilder {
    functions: Map<FnName, Function>,
    globals: Map<GlobalName, Global>,
    next_fn: u32,
    next_global: u32,
}

impl ProgramBuilder {
    pub fn new() -> ProgramBuilder {
        ProgramBuilder {
            functions: Default::default(),
            globals: Default::default(),
            next_fn: 0,
            next_global: 0,
        }
    }

    pub fn finish_program(self, start_function: FnName) -> Program {
        Program { functions: self.functions, start: start_function, globals: self.globals }
    }

    pub fn declare_function(&mut self) -> FunctionBuilder {
        let name = FnName(Name::from_internal(self.next_fn));
        self.next_fn += 1;
        FunctionBuilder::new(name)
    }

    #[track_caller]
    pub fn finish_function(&mut self, f: FunctionBuilder) -> FnName {
        let name = f.name();
        let f = f.finish_function();
        self.functions.try_insert(name, f).unwrap();
        name
    }
}

pub struct FunctionBuilder {
    name: FnName,
    locals: Map<LocalName, Type>,
    args: List<LocalName>,
    blocks: Map<BbName, BasicBlock>,

    start: BbName,
    ret: Option<LocalName>,

    cur_block: Option<CurBlock>,

    next_block: u32,
    next_local: u32,
}

impl FunctionBuilder {
    fn new(name: FnName) -> FunctionBuilder {
        let mut fb = FunctionBuilder {
            name,
            locals: Default::default(),
            blocks: Default::default(),
            args: Default::default(),
            start: BbName(Name::from_internal(0)),
            ret: None,
            cur_block: None,
            next_block: 0,
            next_local: 0,
        };
        // prepare the starting block
        let start_block = fb.declare_block();
        // Make sure we set `start` correctly above.
        assert_eq!(start_block, fb.start);
        fb.set_cur_block(start_block);
        fb
    }

    fn declare_block(&mut self) -> BbName {
        let name = BbName(Name::from_internal(self.next_block));
        self.next_block += 1;
        name
    }

    fn set_cur_block(&mut self, name: BbName) {
        if self.blocks.contains_key(name) {
            panic!("Already inserted a block with this name.")
        }
        self.cur_block = match self.cur_block {
            None => Some(CurBlock::new(name)),
            Some(_) =>
                panic!("There is an unfinished current block. Cannot set a new current block."),
        };
    }

    fn cur_block(&mut self) -> &mut CurBlock {
        self.cur_block
            .as_mut()
            .expect("There is no current block. Cannot insert statement/terminator.")
    }

    #[track_caller]
    fn finish_function(mut self) -> Function {
        if self.cur_block.is_some() {
            panic!(
                "Function has an unfinished block. You need to return or exit from the last block."
            )
        }

        // Default return type to `()`
        if self.ret.is_none() {
            self.declare_ret::<()>();
        }

        Function {
            locals: self.locals,
            args: self.args,
            ret: self.ret.unwrap(),
            calling_convention: CallingConvention::C,
            blocks: self.blocks,
            start: self.start,
        }
    }

    pub fn name(&self) -> FnName {
        self.name
    }

    fn fresh_local_name(&mut self) -> LocalName {
        let name = LocalName(Name::from_internal(self.next_local));
        self.next_local += 1;
        name
    }

    pub fn declare_local<T: TypeConv>(&mut self) -> PlaceExpr {
        let name = self.fresh_local_name();
        self.locals.try_insert(name, T::get_type()).unwrap();
        local_by_name(name)
    }

    pub fn declare_local_with_ty(&mut self, t: Type) -> PlaceExpr {
        let name = self.fresh_local_name();
        self.locals.try_insert(name, t).unwrap();
        local_by_name(name)
    }

    #[track_caller]
    pub fn declare_ret<T: TypeConv>(&mut self) -> PlaceExpr {
        let name = match self.ret {
            Some(_) => panic!("Ret local already set."),
            None => self.fresh_local_name(),
        };
        self.locals.try_insert(name, T::get_type()).unwrap();
        self.ret = Some(name);
        local_by_name(name)
    }

    pub fn declare_arg<T: TypeConv>(&mut self) -> PlaceExpr {
        let name = self.fresh_local_name();
        self.locals.try_insert(name, T::get_type()).unwrap();
        self.args.push(name);
        local_by_name(name)
    }
}

struct CurBlock {
    statements: List<Statement>,
    name: BbName,
}

impl CurBlock {
    pub fn new(name: BbName) -> CurBlock {
        CurBlock { statements: Default::default(), name }
    }
}

fn bbname_into_u32(name: BbName) -> u32 {
    let BbName(name) = name;
    name.get_internal()
}

pub fn align(bytes: impl Into<Int>) -> Align {
    let bytes = bytes.into();
    Align::from_bytes(bytes).unwrap()
}

pub fn size(bytes: impl Into<Int>) -> Size {
    Size::from_bytes(bytes).unwrap()
}

pub fn sized_size(bytes: impl Into<Int>) -> SizeStrategy {
    SizeStrategy::Sized(Size::from_bytes(bytes).unwrap())
}

pub fn offset(bytes: impl Into<Int>) -> Offset {
    size(bytes)
}

// The first function in `fns` is the start function of the program.
pub fn program_with_globals(fns: &[Function], globals: &[Global]) -> Program {
    let functions: Map<FnName, Function> = fns
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let name = FnName(Name::from_internal(i as _));
            (name, *f)
        })
        .collect();

    let globals: Map<GlobalName, Global> = globals
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let name = GlobalName(Name::from_internal(i as _));
            (name, *g)
        })
        .collect();

    Program { functions, start: FnName(Name::from_internal(0)), globals }
}

// The first function in `fns` is the start function of the program.
pub fn program(fns: &[Function]) -> Program {
    program_with_globals(fns, &[])
}

// Generates a small program with a single basic block.
pub fn small_program(locals: &[Type], statements: &[Statement]) -> Program {
    let b = block(statements, exit());
    let f = function(Ret::No, 0, locals, &[b]);

    program(&[f])
}
