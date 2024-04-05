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

    pub fn get_fn_name_smir(&mut self, key: smir::Instance) -> FnName {
        self.get_fn_name(smir::internal(self.tcx, key))
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
