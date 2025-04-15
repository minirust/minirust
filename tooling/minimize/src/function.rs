use crate::*;

/// Data regarding the currently translated function.
pub struct FnCtxt<'cx, 'tcx> {
    /// the body we intend to translate. substitutions are already applied.
    pub body: rs::Body<'tcx>,
    /// the ABI of this function
    abi: &'tcx rs::FnAbi<'tcx, rs::Ty<'tcx>>,

    /// the list of local variable declarations (StableMIR) used to retrieve the type of some
    /// SMIR constructs.
    pub locals_smir: Vec<smir::LocalDecl>,

    pub cx: &'cx mut Ctxt<'tcx>,

    /// associate names for each mir Local.
    pub local_name_map: HashMap<rs::Local, LocalName>,

    /// associate names for each basic block.
    pub bb_name_map: HashMap<rs::BasicBlock, BbName>,

    /// The next free number that can be used as name for a basic block
    next_bb: u32,

    pub locals: Map<LocalName, Type>,
    pub blocks: Map<BbName, BasicBlock>,
}

impl<'cx, 'tcx> std::ops::Deref for FnCtxt<'cx, 'tcx> {
    type Target = Ctxt<'tcx>;

    fn deref(&self) -> &Self::Target {
        &self.cx
    }
}

impl<'cx, 'tcx> std::ops::DerefMut for FnCtxt<'cx, 'tcx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cx
    }
}

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn new(instance: rs::Instance<'tcx>, cx: &'cx mut Ctxt<'tcx>) -> Self {
        let body = cx.tcx.instance_mir(instance.def);
        // We eagerly instantiate everything upfront once.
        // Then nothing else has to worry about generics.
        let body = cx.tcx.instantiate_and_normalize_erasing_regions(
            instance.args,
            rs::ParamEnv::reveal_all(),
            rs::EarlyBinder::bind(body.clone()),
        );
        let abi = cx
            .tcx
            .fn_abi_of_instance(rs::ParamEnv::reveal_all().and((instance, rs::List::empty())))
            .unwrap();
        let locals_smir = smir::stable(&body).locals().to_vec();

        FnCtxt {
            body,
            abi,
            cx,
            local_name_map: Default::default(),
            bb_name_map: Default::default(),
            locals: Default::default(),
            blocks: Default::default(),
            locals_smir,
            next_bb: 0,
        }
    }

    pub fn fresh_bb_name(&mut self) -> BbName {
        let name = self.next_bb;
        self.next_bb = name.checked_add(1).unwrap();
        BbName(Name::from_internal(name))
    }

    /// translates a function body.
    /// Any fn calls occuring during this translation will be added to the `FnNameMap`.
    pub fn translate(mut self) -> Function {
        // associate names for each mir BB.
        for bb_id in self.body.basic_blocks.indices() {
            if self.body.basic_blocks[bb_id].is_cleanup {
                // We don't support unwinding, so we don't translate cleanup blocks.
                continue;
            }
            let bb_name = self.fresh_bb_name();
            self.bb_name_map.insert(bb_id, bb_name);
        }

        for local_id in self.body.local_decls.indices() {
            let local_name = self.local_name_map.len(); // .len() is the next free index
            let local_name = LocalName(Name::from_internal(local_name as u32));
            self.local_name_map.insert(local_id, local_name);
        }

        // convert mirs Local-types to minirust.
        for (id, local_name) in &self.local_name_map {
            let local_decl = &self.body.local_decls[*id];
            let span = local_decl.source_info.span;
            let ty = self.cx.translate_ty(local_decl.ty, span);
            self.locals.insert(*local_name, ty);
        }

        // the number of locals which are implicitly storage live.
        let free_argc = self.body.arg_count + 1;

        // add init basic block
        let init_bb = self.fresh_bb_name();

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
            terminator: Terminator::Goto(self.bb_name_map[&rs::mir::START_BLOCK]),
            kind: BbKind::Regular,
        };
        self.blocks.insert(init_bb, init_blk);

        // convert MIR BBs to minirust.
        for (id, bb_name) in self.bb_name_map.clone() {
            let bb_data = &self.body.basic_blocks[id].clone(); // TODO fix clone
            self.translate_bb(bb_name, bb_data);
        }

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
            calling_convention: translate_calling_convention(self.abi.conv),
        };

        f
    }
}
