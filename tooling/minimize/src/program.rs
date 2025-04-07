use crate::*;

pub struct Ctxt<'tcx> {
    pub tcx: rs::TyCtxt<'tcx>,

    /// maps Rust function calls to MiniRust FnNames.
    pub fn_name_map: HashMap<rs::Instance<'tcx>, FnName>,

    /// maps Rust traits to MiniRust TraitNames and those to sets of methods.
    pub trait_map: HashMap<&'tcx rs::List<rs::PolyExistentialPredicate<'tcx>>, TraitName>,
    pub traits: Map<TraitName, Set<TraitMethodName>>,

    /// maps Rust vtables to MiniRust VTableNames and those to Vtables.
    pub vtable_map:
        HashMap<(rs::Ty<'tcx>, &'tcx rs::List<rs::PolyExistentialPredicate<'tcx>>), VTableName>,
    pub vtables: Map<VTableName, VTable>,

    /// Stores which AllocId evaluates to which GlobalName.
    pub alloc_map: HashMap<rs::AllocId, GlobalName>,

    pub globals: Map<GlobalName, Global>,

    pub functions: Map<FnName, Function>,

    pub ty_cache: HashMap<rs::Ty<'tcx>, Type>,
}

impl<'tcx> Ctxt<'tcx> {
    pub fn new(tcx: rs::TyCtxt<'tcx>) -> Self {
        // Ensure consistency with the DefaultTarget
        let dl = tcx.data_layout();
        assert_eq!(DefaultTarget::PTR_SIZE, translate_size(dl.pointer_size));
        assert_eq!(DefaultTarget::PTR_ALIGN, translate_align(dl.pointer_align.abi));
        assert_eq!(DefaultTarget::ENDIANNESS, match dl.endian {
            rs::abi::Endian::Little => Endianness::LittleEndian,
            rs::abi::Endian::Big => Endianness::BigEndian,
        });
        for rs_int_ty in [rs::abi::I8, rs::abi::I16, rs::abi::I32, rs::abi::I64, rs::abi::I128] {
            let size = translate_size(rs_int_ty.size());
            // Rust alignment:
            let align = translate_align(rs_int_ty.align(dl).abi);
            // MiniRust alignment:
            // Signedness does not matter, we just care to compare the alignments.
            let int_ty = IntType { size, signed: Signedness::Unsigned };
            assert_eq!(
                int_ty.align::<DefaultTarget>(),
                align,
                "{rs_int_ty:?} seem to have the wrong alignment"
            );
        }

        Ctxt {
            tcx,
            fn_name_map: Default::default(),
            trait_map: Default::default(),
            vtable_map: Default::default(),
            traits: Default::default(),
            vtables: Default::default(),
            alloc_map: Default::default(),
            globals: Default::default(),
            functions: Default::default(),
            ty_cache: Default::default(),
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

        Program {
            start,
            functions: self.functions,
            globals: self.globals,
            vtables: self.vtables,
            traits: self.traits,
        }
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
    pub fn rs_layout_of_smir(&self, ty: smir::Ty) -> rs::Layout<'tcx> {
        self.rs_layout_of(smir::internal(self.tcx, ty))
    }
}

fn mk_start_fn(entry: u32) -> Function {
    let b0_name = BbName(Name::from_internal(0));
    let b1_name = BbName(Name::from_internal(1));
    let l0_name = LocalName(Name::from_internal(0));

    let b0 = BasicBlock {
        statements: List::new(),
        terminator: Terminator::Call {
            callee: build::fn_ptr_internal(entry),
            calling_convention: CallingConvention::Rust,
            arguments: List::new(),
            ret: build::unit_place(),
            next_block: Some(b1_name),
        },
    };

    let b1 = BasicBlock {
        statements: List::new(),
        terminator: Terminator::Intrinsic {
            intrinsic: IntrinsicOp::Exit,
            arguments: List::new(),
            ret: build::unit_place(),
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
