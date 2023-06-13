use crate::*;

pub struct Ctxt<'tcx> {
    pub tcx: rs::TyCtxt<'tcx>,

    /// maps Rust function calls to MiniRust FnNames.
    pub fn_name_map: HashMap<(rs::DefId, rs::SubstsRef<'tcx>), FnName>,

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
        let substs_ref: rs::SubstsRef<'tcx> = self.tcx.mk_substs(&[]);
        let entry_name = FnName(Name::from_internal(0));

        self.fn_name_map.insert((entry, substs_ref), entry_name);

        // take any not-yet-implemented function:
        while let Some(fn_name) = self
            .fn_name_map
            .values()
            .find(|k| !self.functions.contains_key(**k))
            .copied()
        {
            let (def_id, substs_ref) = self
                .fn_name_map
                .iter()
                .find(|(_, f)| **f == fn_name)
                .map(|(r, _)| r)
                .unwrap();

            let f = FnCtxt::new(*def_id, substs_ref, &mut self).translate();
            self.functions.insert(fn_name, f);
        }

        let number_of_fns = self.fn_name_map.len();

        // add a `start` function, which calls `entry`.
        let start = FnName(Name::from_internal(number_of_fns as _));
        self.functions.insert(start, mk_start_fn(0));

        Program {
            start,
            functions: self.functions,
            globals: self.globals,
        }
    }
}

fn mk_start_fn(entry: u32) -> Function {
    let b0_name = BbName(Name::from_internal(0));
    let b1_name = BbName(Name::from_internal(1));

    let b0 = BasicBlock {
        statements: List::new(),
        terminator: Terminator::Call {
            callee: build::fn_ptr(entry),
            arguments: List::new(),
            ret: None,
            next_block: Some(b1_name),
        },
    };

    let b1 = BasicBlock {
        statements: List::new(),
        terminator: Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Exit,
            arguments: List::new(),
            ret: None,
            next_block: None,
        },
    };

    let mut blocks = Map::new();
    blocks.insert(b0_name, b0);
    blocks.insert(b1_name, b1);

    Function {
        locals: Map::new(),
        args: List::new(),
        ret: None,
        blocks,
        start: b0_name,
    }
}

/// data regarding the currently translated function.
pub struct FnCtxt<'cx, 'tcx> {
    // the body we intend to translate.
    pub body: rs::Body<'tcx>,
    pub def_id: rs::DefId,
    pub substs_ref: rs::SubstsRef<'tcx>,

    pub cx: &'cx mut Ctxt<'tcx>,

    // associate names for each mir Local.
    pub local_name_map: HashMap<rs::Local, LocalName>,

    // associate names for each basic block.
    pub bb_name_map: HashMap<rs::BasicBlock, BbName>,

    pub locals: Map<LocalName, PlaceType>,
    pub blocks: Map<BbName, BasicBlock>,
}

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn new(
        def_id: rs::DefId,
        substs_ref: rs::SubstsRef<'tcx>,
        cx: &'cx mut Ctxt<'tcx>,
    ) -> Self {
        let body = cx.tcx.optimized_mir(def_id);
        let body = cx.tcx.subst_and_normalize_erasing_regions(
            substs_ref,
            rs::ParamEnv::empty(),
            rs::EarlyBinder::bind(body.clone()),
        );

        FnCtxt {
            body,
            def_id,
            substs_ref,
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
            self.locals
                .insert(*local_name, translate_local(local_decl, self.cx.tcx));
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
            let bb = translate_bb(bb_data, &mut self);
            self.blocks.insert(bb_name, bb);
        }
        self.blocks.insert(init_bb, init_blk);

        let (ret_abi, arg_abis) = calc_abis(self.def_id, self.substs_ref, self.cx.tcx);

        // "The first local is the return value pointer, followed by arg_count locals for the function arguments, followed by any user-declared variables and temporaries."
        // - https://doc.rust-lang.org/stable/nightly-rustc/rustc_middle/mir/struct.Body.html
        let ret = Some((LocalName(Name::from_internal(0)), ret_abi));

        let mut args = List::default();
        for (i, arg_abi) in arg_abis.iter().enumerate() {
            let i = i + 1; // this starts counting with 1, as id 0 is the return value of the function.
            let local_name = LocalName(Name::from_internal(i as _));
            args.push((local_name, arg_abi));
        }

        let f = Function {
            locals: self.locals,
            args,
            ret,
            blocks: self.blocks,
            start: init_bb,
        };

        f
    }
}

// TODO simplify this function.
pub fn calc_abis<'tcx>(
    def_id: rs::DefId,
    substs_ref: rs::SubstsRef<'tcx>,
    tcx: rs::TyCtxt<'tcx>,
) -> (/*ret:*/ ArgAbi, /*args:*/ List<ArgAbi>) {
    let instance = rs::Instance::resolve(tcx, rs::ParamEnv::empty(), def_id, substs_ref).unwrap().unwrap();
    let fn_abi = tcx.fn_abi_of_instance(rs::ParamEnv::empty().and((instance, rs::List::empty()))).unwrap();
    let ret = translate_arg_abi(&fn_abi.ret);
    let args = fn_abi.args.iter().map(|x| translate_arg_abi(x)).collect();
    (ret, args)
}

// TODO extend when Minirust has a more sophisticated ArgAbi
pub fn translate_arg_abi<'a, T>(arg_abi: &rs::ArgAbi<'a, T>) -> ArgAbi {
    if let rs::PassMode::Direct(attrs) = arg_abi.mode {
        // FIXME for some reason, this is never true.
        if attrs.regular.contains(rs::ArgAttribute::InReg) {
            return ArgAbi::Register;
        }
    }

    let size = arg_abi.layout.size;
    let align = arg_abi.layout.align.abi;
    ArgAbi::Stack(translate_size(size), translate_align(align))
}

fn translate_local<'tcx>(local: &rs::LocalDecl<'tcx>, tcx: rs::TyCtxt<'tcx>) -> PlaceType {
    let ty = translate_ty(local.ty, tcx);

    // generics have already been resolved before, so `ParamEnv::empty()` is correct.
    let a = rs::ParamEnv::empty().and(local.ty);
    let layout = tcx.layout_of(a).unwrap().layout;
    let align = layout.align().abi;
    let align = translate_align(align);

    PlaceType { ty, align }
}

pub fn translate_align(align: rs::Align) -> Align {
    Align::from_bytes(align.bytes()).unwrap()
}
