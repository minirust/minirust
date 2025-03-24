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
    vtables: Map<VTableName, VTable>,
    traits: Map<TraitName, Set<TraitMethodName>>,
    next_fn: u32,
    next_global: u32,
    next_vtable: u32,
    next_trait: u32,
}

impl ProgramBuilder {
    pub fn new() -> ProgramBuilder {
        ProgramBuilder {
            functions: Default::default(),
            globals: Default::default(),
            vtables: Default::default(),
            traits: Default::default(),
            next_fn: 0,
            next_global: 0,
            next_vtable: 0,
            next_trait: 0,
        }
    }

    pub fn finish_program(self, start_function: FnName) -> Program {
        Program {
            functions: self.functions,
            start: start_function,
            globals: self.globals,
            traits: self.traits,
            vtables: self.vtables,
        }
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

    pub fn declare_vtable_for_ty(&mut self, trait_name: TraitName, ty: Type) -> VTableBuilder {
        self.declare_vtable(
            trait_name,
            ty.layout::<DefaultTarget>().expect_size("only sized types can be trait objects"),
            ty.layout::<DefaultTarget>().expect_align("only sized types can be trait objects"),
        )
    }

    pub fn declare_vtable(
        &mut self,
        trait_name: TraitName,
        size: Size,
        align: Align,
    ) -> VTableBuilder {
        let name = VTableName(Name::from_internal(self.next_vtable));
        self.next_vtable += 1;
        VTableBuilder::new(trait_name, name, size, align)
    }

    #[track_caller]
    pub fn finish_vtable(&mut self, v: VTableBuilder) -> VTableName {
        let name = v.name();
        let vtable = v.finish_vtable();
        // Check that this vtable has all the methods required by the trait.
        assert_eq!(
            self.traits.get(vtable.trait_name).expect("TraitName must have been declared"),
            vtable.methods.keys().collect(),
            "vtable does not declare the right set of methods",
        );
        self.vtables.try_insert(name, vtable).unwrap();
        name
    }

    pub fn declare_trait(&mut self) -> TraitBuilder {
        let name = TraitName(Name::from_internal(self.next_trait));
        self.next_trait += 1;
        TraitBuilder::new(name)
    }

    #[track_caller]
    pub fn finish_trait(&mut self, t: TraitBuilder) -> TraitName {
        let name = t.name();
        let methods = t.finish_trait();
        self.traits.try_insert(name, methods).unwrap();
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
        fb.set_cur_block(start_block, BbKind::Regular);
        fb
    }

    fn declare_block(&mut self) -> BbName {
        let name = BbName(Name::from_internal(self.next_block));
        self.next_block += 1;
        name
    }

    fn set_cur_block(&mut self, name: BbName, kind: BbKind) {
        if self.blocks.contains_key(name) {
            panic!("Already inserted a block with this name.")
        }
        self.cur_block = match self.cur_block {
            None => Some(CurBlock::new(name, kind)),
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
        self.declare_arg_with_ty(T::get_type())
    }

    pub fn declare_arg_with_ty(&mut self, ty: Type) -> PlaceExpr {
        let name = self.fresh_local_name();
        self.locals.try_insert(name, ty).unwrap();
        self.args.push(name);
        local_by_name(name)
    }

    pub fn cleanup<F>(&mut self, cleanup_builder: F) -> BbName
    where
        F: Fn(&mut Self),
    {
        let mut cur_block = self.cur_block.take();
        let cleanup_block = self.declare_block();
        self.set_cur_block(cleanup_block, BbKind::Cleanup);
        cleanup_builder(self);

        if self.cur_block.is_some() {
            panic!("The cleanup block is unfinished. The block needs to end with a Terminator.");
        }
        self.cur_block = cur_block.take();
        cleanup_block
    }

    pub fn cleanup_resume(&mut self) -> BbName {
        self.cleanup(|f| {
            f.resume_unwind();
        })
    }

    pub fn terminate<F>(&mut self, terminat_builer: F) -> BbName
    where
        F: Fn(&mut Self),
    {
        let mut cur_block = self.cur_block.take();
        let terminate_block = self.declare_block();
        self.set_cur_block(terminate_block, BbKind::Terminate);
        terminat_builer(self);
        // Add Unreachable if no terminator is specified.
        if self.cur_block.is_some() {
            panic!("The terminate block is unfinished. The block needs to end with a Terminator.");
        }
        self.cur_block = cur_block.take();
        terminate_block
    }
}

pub struct VTableBuilder {
    trait_name: TraitName,
    name: VTableName,
    size: Size,
    align: Align,
    methods: Map<TraitMethodName, FnName>,
}

impl VTableBuilder {
    fn new(trait_name: TraitName, name: VTableName, size: Size, align: Align) -> VTableBuilder {
        VTableBuilder { trait_name, name, size, align, methods: Map::new() }
    }

    pub fn name(&self) -> VTableName {
        self.name
    }

    pub fn add_method(&mut self, index: TraitMethodName, func: FnName) {
        self.methods.insert(index, func);
    }

    #[track_caller]
    fn finish_vtable(self) -> VTable {
        VTable {
            trait_name: self.trait_name,
            size: self.size,
            align: self.align,
            methods: self.methods,
        }
    }
}

pub struct TraitBuilder {
    name: TraitName,
    next_method: u32,
    method_names: Set<TraitMethodName>,
}

impl TraitBuilder {
    fn new(name: TraitName) -> TraitBuilder {
        TraitBuilder { name, next_method: 0, method_names: Set::new() }
    }

    pub fn name(&self) -> TraitName {
        self.name
    }

    pub fn declare_method(&mut self) -> TraitMethodName {
        let idx = self.next_method;
        let name = TraitMethodName(Name::from_internal(idx));
        self.next_method += 1;
        self.method_names.insert(name);
        name
    }

    #[track_caller]
    fn finish_trait(self) -> Set<TraitMethodName> {
        self.method_names
    }
}

struct CurBlock {
    statements: List<Statement>,
    name: BbName,
    kind: BbKind,
}

impl CurBlock {
    pub fn new(name: BbName, kind: BbKind) -> CurBlock {
        CurBlock { statements: Default::default(), name, kind }
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

    Program {
        functions,
        start: FnName(Name::from_internal(0)),
        globals,
        traits: Default::default(),
        vtables: Default::default(),
    }
}

// The first function in `fns` is the start function of the program.
pub fn program(fns: &[Function]) -> Program {
    program_with_globals(fns, &[])
}

// Generates a small program with a single basic block.
pub fn small_program(locals: &[Type], statements: &[Statement]) -> Program {
    let b = block(statements, exit(), BbKind::Regular);
    let f = function(Ret::No, 0, locals, &[b]);

    program(&[f])
}
