use crate::*;

pub struct Ctxt<'tcx> {
    pub tcx: rs::TyCtxt<'tcx>,

    /// maps Rust function calls to MiniRust FnNames.
    pub fn_name_map: HashMap<rs::Instance<'tcx>, FnName>,

    /// Stores which AllocId evaluates to which GlobalName.
    /// Note that not every AllocId and not every GlobalName is coming up in this map (for example constants are missing).
    pub alloc_map: HashMap<rs::AllocId, GlobalName>,

    pub globals: Map<GlobalName, Global>,

    pub functions: Map<FnName, Function>,
}

impl<'tcx> Ctxt<'tcx> {
    pub fn new(tcx: rs::TyCtxt<'tcx>) -> Self {
        Ctxt {
            tcx,
            fn_name_map: Default::default(),
            alloc_map: Default::default(),
            globals: Default::default(),
            functions: Default::default(),
        }
    }

    pub fn translate(mut self) -> Program {
        let (entry, _ty) = self.tcx.entry_fn(()).unwrap();
        let entry_instance = rs::Instance::mono(self.tcx, entry);
        let entry_name = FnName(Name::from_internal(0));

        self.fn_name_map.insert(entry_instance, entry_name);

        // This is the main monomorphization loop.
        // take any not-yet-implemented function:
        while let Some(fn_name) =
            self.fn_name_map.values().find(|k| !self.functions.contains_key(**k)).copied()
        {
            let instance =
                self.fn_name_map.iter().find(|(_, f)| **f == fn_name).map(|(r, _)| r).unwrap();

            let f = FnCtxt::new(*instance, &mut self).translate();
            self.functions.insert(fn_name, f);
        }

        let number_of_fns = self.fn_name_map.len();

        // add a `start` function, which calls `entry`.
        let start = FnName(Name::from_internal(number_of_fns as _));
        self.functions.insert(start, mk_start_fn(0));

        Program { start, functions: self.functions, globals: self.globals }
    }

    // Returns FnName associated with some key. If it does not exist it creates a new one.
    pub fn get_fn_name(&mut self, key: rs::Instance<'tcx>) -> FnName {
        // Used as the fn name if it is not named yet.
        let len = self.fn_name_map.len();

        *self.fn_name_map.entry(key).or_insert_with(|| FnName(Name::from_internal(len as _)))
    }

    pub fn rs_layout_of(&self, ty: rs::Ty<'tcx>) -> rs::Layout<'tcx> {
        self.tcx.layout_of(rs::ParamEnv::reveal_all().and(ty)).unwrap().layout
    }
}

fn mk_start_fn(entry: u32) -> Function {
    let b0_name = BbName(Name::from_internal(0));
    let b1_name = BbName(Name::from_internal(1));
    let l0_name = LocalName(Name::from_internal(0));

    let b0 = BasicBlock {
        statements: List::new(),
        terminator: Terminator::Call {
            callee: build::fn_ptr_conv(entry, CallingConvention::Rust),
            arguments: List::new(),
            ret: build::zst_place(),
            next_block: Some(b1_name),
        },
    };

    let b1 = BasicBlock {
        statements: List::new(),
        terminator: Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Exit,
            arguments: List::new(),
            ret: build::zst_place(),
            next_block: None,
        },
    };

    let mut blocks = Map::new();
    blocks.insert(b0_name, b0);
    blocks.insert(b1_name, b1);

    let mut locals = Map::new();
    locals.insert(l0_name, <()>::get_type());

    Function {
        locals,
        args: List::new(),
        ret: l0_name,
        blocks,
        start: b0_name,
        calling_convention: CallingConvention::C,
    }
}

/// data regarding the currently translated function.
pub struct FnCtxt<'cx, 'tcx> {
    /// the body we intend to translate. substitutions are already applied.
    pub body: rs::Body<'tcx>,
    /// where the body comes from.
    pub instance: rs::Instance<'tcx>,

    pub cx: &'cx mut Ctxt<'tcx>,

    // associate names for each mir Local.
    pub local_name_map: HashMap<rs::Local, LocalName>,

    // associate names for each basic block.
    pub bb_name_map: HashMap<rs::BasicBlock, BbName>,

    pub locals: Map<LocalName, Type>,
    pub blocks: Map<BbName, BasicBlock>,
}

impl<'cx, 'tcx> std::ops::Deref for FnCtxt<'cx, 'tcx> {
    type Target = Ctxt<'tcx>;

    fn deref(&self) -> &Self::Target {
        &self.cx
    }
}

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn new(instance: rs::Instance<'tcx>, cx: &'cx mut Ctxt<'tcx>) -> Self {
        let body = cx.tcx.optimized_mir(instance.def_id());
        // We eagerly instantiate everything upfront once.
        // Then nothing else has to worry about generics.
        let body = cx.tcx.instantiate_and_normalize_erasing_regions(
            instance.args,
            rs::ParamEnv::reveal_all(),
            rs::EarlyBinder::bind(body.clone()),
        );

        FnCtxt {
            body,
            instance,
            cx,
            local_name_map: Default::default(),
            bb_name_map: Default::default(),
            locals: Default::default(),
            blocks: Default::default(),
        }
    }

    /// translates a function body.
    /// Any fn calls occuring during this translation will be added to the `FnNameMap`.
    pub fn translate(mut self) -> Function {
        let abi = self
            .cx
            .tcx
            .fn_abi_of_instance(rs::ParamEnv::reveal_all().and((self.instance, rs::List::empty())))
            .unwrap();

        // associate names for each mir BB.
        for bb_id in self.body.basic_blocks.indices() {
            let bb_name = self.bb_name_map.len(); // .len() is the next free index
            let bb_name = BbName(Name::from_internal(bb_name as u32));
            self.bb_name_map.insert(bb_id, bb_name);
        }

        // bb with id 0 is the start block:
        // see https://doc.rust-lang.org/stable/nightly-rustc/src/rustc_middle/mir/mod.rs.html#1014-1042
        let rs_start = BbName(Name::from_internal(0));

        for local_id in self.body.local_decls.indices() {
            let local_name = self.local_name_map.len(); // .len() is the next free index
            let local_name = LocalName(Name::from_internal(local_name as u32));
            self.local_name_map.insert(local_id, local_name);
        }

        // convert mirs Local-types to minirust.
        for (id, local_name) in &self.local_name_map {
            let local_decl = &self.body.local_decls[*id];
            self.locals.insert(*local_name, self.translate_ty(local_decl.ty));
        }

        // the number of locals which are implicitly storage live.
        let free_argc = self.body.arg_count + 1;

        // add init basic block
        let init_bb = BbName(Name::from_internal(self.bb_name_map.len() as u32));

        // this block allocates all "always_storage_live_locals",
        // except for those which are implicitly storage live in Minirust;
        // like the return local and function args.
        let init_blk = BasicBlock {
            statements: rs::always_storage_live_locals(&self.body)
                .iter()
                .map(|loc| self.local_name_map[&loc])
                .filter(|LocalName(i)| i.get_internal() as usize >= free_argc)
                .map(Statement::StorageLive)
                .collect(),
            terminator: Terminator::Goto(rs_start),
        };

        // convert mirs BBs to minirust.
        for (id, bb_name) in self.bb_name_map.clone() {
            // TODO fix clone
            let bb_data = &self.body.basic_blocks[id].clone(); // TODO fix clone
            let bb = self.translate_bb(bb_data);
            self.blocks.insert(bb_name, bb);
        }
        self.blocks.insert(init_bb, init_blk);

        // "The first local is the return value pointer, followed by arg_count locals for the function arguments, followed by any user-declared variables and temporaries."
        // - https://doc.rust-lang.org/stable/nightly-rustc/rustc_middle/mir/struct.Body.html
        let ret = LocalName(Name::from_internal(0));

        let mut args = List::default();
        for i in 0..self.body.arg_count {
            let i = i + 1; // this starts counting with 1, as id 0 is the return value of the function.
            let local_name = LocalName(Name::from_internal(i as _));
            args.push(local_name);
        }

        let f = Function {
            locals: self.locals,
            args,
            ret,
            blocks: self.blocks,
            start: init_bb,
            calling_convention: translate_calling_convention(abi.conv),
        };

        f
    }
}

pub fn translate_calling_convention(conv: rs::Conv) -> CallingConvention {
    match conv {
        rs::Conv::C => CallingConvention::C,
        rs::Conv::Rust => CallingConvention::Rust,
        _ => todo!(),
    }
}

pub fn translate_align(align: rs::Align) -> Align {
    Align::from_bytes(align.bytes()).unwrap()
}
