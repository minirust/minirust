use crate::*;

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn translate_const(&mut self, c: &rs::mir::Const<'tcx>) -> ValueExpr {
        match c {
            rs::mir::Const::Ty(_) => panic!("not supported!"),
            rs::mir::Const::Unevaluated(uneval, ty) => self.translate_const_uneval(uneval, *ty),
            rs::mir::Const::Val(val, ty) => self.translate_const_val(val, *ty),
        }
    }

    fn translate_const_val(&mut self, val: &rs::ConstValue<'tcx>, ty: rs::Ty<'tcx>) -> ValueExpr {
        let ty = self.translate_ty(ty);

        let constant = match ty {
            Type::Int(int_ty) => {
                let val = val.try_to_scalar_int().unwrap();
                let int: Int = match int_ty.signed {
                    Signed => val.try_to_int(val.size()).unwrap().into(),
                    Unsigned => val.try_to_uint(val.size()).unwrap().into(),
                };
                Constant::Int(int)
            }
            Type::Bool => Constant::Bool(val.try_to_bool().unwrap()),
            Type::Tuple { fields, .. } if fields.is_empty() => {
                return ValueExpr::Tuple(List::new(), ty);
            }
            // A `static`
            Type::Ptr(_) => {
                let (alloc_id, offset) =
                    val.try_to_scalar().unwrap().to_pointer(&self.tcx).unwrap().into_parts();
                let alloc_id = alloc_id.expect("no alloc id?").alloc_id();
                let rel = self.translate_relocation(alloc_id, offset);
                Constant::GlobalPointer(rel)
            }
            ty => panic!("unsupported type for `ConstVal`: {:?}", ty),
        };
        ValueExpr::Constant(constant, ty)
    }

    fn translate_const_uneval(
        &mut self,
        uneval: &rs::UnevaluatedConst<'tcx>,
        ty: rs::Ty<'tcx>,
    ) -> ValueExpr {
        let Ok(Some(instance)) =
            rs::Instance::resolve(self.tcx, rs::ParamEnv::reveal_all(), uneval.def, uneval.args)
        else {
            panic!("can't resolve unevaluated const!")
        };
        let cid = rs::GlobalId { instance, promoted: uneval.promoted };
        let alloc = self.tcx.eval_to_allocation_raw(rs::ParamEnv::reveal_all().and(cid)).unwrap();
        let name = self.translate_alloc_id(alloc.alloc_id);
        let offset = Offset::ZERO;

        let rel = Relocation { name, offset };
        self.relocation_to_value_expr(rel, ty)
    }

    fn relocation_to_value_expr(&mut self, rel: Relocation, ty: rs::Ty<'tcx>) -> ValueExpr {
        let expr = Constant::GlobalPointer(rel);

        let ty = self.translate_ty(ty);
        let ptr_ty = Type::Ptr(PtrType::Raw);

        let expr = ValueExpr::Constant(expr, ptr_ty);
        let expr = PlaceExpr::Deref { operand: GcCow::new(expr), ty };
        ValueExpr::Load { source: GcCow::new(expr) }
    }

    fn translate_relocation(&mut self, alloc_id: rs::AllocId, offset: rs::Size) -> Relocation {
        let name = self.translate_alloc_id(alloc_id);
        let offset = translate_size(offset);
        Relocation { name, offset }
    }

    // calls `translate_const_allocation` with the allocation of alloc_id,
    // and adds the alloc_id and its newly-created global to alloc_map.
    fn translate_alloc_id(&mut self, alloc_id: rs::AllocId) -> GlobalName {
        if let Some(x) = self.alloc_map.get(&alloc_id) {
            return *x;
        }

        let name = self.fresh_global_name();
        self.cx.alloc_map.insert(alloc_id, name);

        let alloc = match self.tcx.global_alloc(alloc_id) {
            rs::GlobalAlloc::Memory(alloc) => alloc,
            rs::GlobalAlloc::Static(def_id) => self.tcx.eval_static_initializer(def_id).unwrap(),
            _ => panic!("unsupported!"),
        };
        self.translate_const_allocation(alloc, name);
        name
    }

    // adds a Global representing this ConstAllocation, and returns the corresponding GlobalName.
    fn translate_const_allocation(
        &mut self,
        allocation: rs::ConstAllocation<'tcx>,
        name: GlobalName,
    ) {
        let allocation = allocation.inner();
        let size = allocation.size();
        let mut bytes: Vec<Option<u8>> = allocation
            .inspect_with_uninit_and_ptr_outside_interpreter(0..size.bytes_usize())
            .iter()
            .copied()
            .map(Some)
            .collect();
        for (i, b) in bytes.iter_mut().enumerate() {
            if !allocation.init_mask().get(rs::Size::from_bytes(i)) {
                *b = None;
            }
        }
        let relocations = allocation
            .provenance()
            .ptrs()
            .iter()
            .map(|&(offset, alloc_id)| {
                // "Note that the bytes of a pointer represent the offset of the pointer.", see https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/interpret/struct.Allocation.html
                // Hence we have to decode them.
                let inner_offset_bytes: &[Option<u8>] = &bytes[offset.bytes() as usize..]
                    [..DefaultTarget::PTR_SIZE.bytes().try_to_usize().unwrap()];
                let inner_offset_bytes: List<u8> =
                    inner_offset_bytes.iter().map(|x| x.unwrap()).collect();
                let inner_offset: Int =
                    DefaultTarget::ENDIANNESS.decode(Unsigned, inner_offset_bytes);
                let inner_offset = rs::Size::from_bytes(inner_offset.try_to_usize().unwrap());
                let relo = self.translate_relocation(alloc_id.alloc_id(), inner_offset);

                let offset = translate_size(offset);
                (offset, relo)
            })
            .collect();
        let align = translate_align(allocation.align);
        let global = Global { bytes: bytes.into_iter().collect(), relocations, align };

        self.cx.globals.insert(name, global);
    }

    fn fresh_global_name(&mut self) -> GlobalName {
        let name = GlobalName(Name::from_internal(self.globals.iter().count() as _)); // TODO use .len() here, if supported
        // the default_global is added so that calling `fresh_global_name` twice returns different names.
        let default_global = Global {
            bytes: Default::default(),
            relocations: Default::default(),
            align: Align::ONE,
        };
        self.cx.globals.insert(name, default_global);
        name
    }
}
