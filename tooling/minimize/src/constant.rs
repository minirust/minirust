use crate::*;

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn translate_const(&mut self, c: &rs::mir::Const<'tcx>, span: rs::Span) -> ValueExpr {
        let val = match c.eval(self.tcx, rs::ParamEnv::reveal_all(), rs::DUMMY_SP) {
            Ok(val) => val,
            Err(_) => rs::span_bug!(span, "const-eval failed"),
        };
        let tcx_at = self.tcx.at(span);
        let (mut ecx, v) =
            rs::mk_eval_cx_for_const_val(tcx_at, rs::ParamEnv::reveal_all(), val, c.ty()).unwrap();
        self.translate_const_val(v, &mut ecx, span)
    }

    pub fn translate_const_smir(&mut self, c: &smir::MirConst, span: rs::Span) -> ValueExpr {
        self.translate_const(&smir::internal(self.tcx, c), span)
    }

    fn translate_const_val(
        &mut self,
        val: rs::OpTy<'tcx>,
        ecx: &mut rs::CompileTimeInterpCx<'tcx>,
        span: rs::Span,
    ) -> ValueExpr {
        let ty = self.translate_ty(val.layout.ty, span);
        match ty {
            Type::Int(int_ty) => {
                let scalar = ecx.read_scalar(&val).unwrap();
                let val: Int = match int_ty.signed {
                    Signed => scalar.to_int(scalar.size()).unwrap().into(),
                    Unsigned => scalar.to_uint(scalar.size()).unwrap().into(),
                };
                ValueExpr::Constant(Constant::Int(val), ty)
            }
            Type::Bool => {
                let val = ecx.read_scalar(&val).unwrap().to_bool().unwrap();
                ValueExpr::Constant(Constant::Bool(val), ty)
            }
            Type::Ptr(ptr_ty) => {
                if let PtrType::FnPtr = ptr_ty {
                    rs::span_bug!(span, "Function pointers are currently not supported")
                }
                let ptr = ecx.read_pointer(&val).unwrap();
                let (prov, offset) = ptr.into_parts();
                let c = match prov {
                    None => {
                        let addr: Int = offset.bytes_usize().into();
                        Constant::PointerWithoutProvenance(addr)
                    }
                    Some(prov) => {
                        let alloc_id = prov.alloc_id();
                        let rel = self.translate_relocation(alloc_id, offset);
                        Constant::GlobalPointer(rel)
                    }
                };
                ValueExpr::Constant(c, ty)
            }
            Type::Tuple { fields, .. } => {
                let mut t: List<ValueExpr> = List::new();
                for (idx, _) in fields.iter().enumerate() {
                    let val = ecx.project_field(&val, idx).unwrap();
                    t.push(self.translate_const_val(val, ecx, span));
                }
                ValueExpr::Tuple(t, ty)
            }
            Type::Enum { variants, discriminant_ty, .. } => {
                // variant_idx is pointer into list of variants
                // while discriminant is the value associated with variant
                let variant_idx = ecx.read_discriminant(&val).unwrap();
                let variant = ecx.project_downcast(&val, variant_idx).unwrap();
                let mut fields: List<ValueExpr> = List::new();
                for i in 0..variant.layout.fields.count() {
                    let field = ecx.project_field(&variant, i).unwrap();
                    let field = self.translate_const_val(field, ecx, span);
                    fields.push(field);
                }

                let discriminant =
                    ecx.discriminant_for_variant(val.layout.ty, variant_idx).unwrap();
                let discriminant = discriminant.to_scalar();
                let discriminant: Int = match discriminant_ty.signed {
                    Signed => discriminant.to_int(discriminant.size()).unwrap().into(),
                    Unsigned => discriminant.to_uint(discriminant.size()).unwrap().into(),
                };
                let variant_ty = variants.get(discriminant).unwrap().ty;
                let data = GcCow::new(ValueExpr::Tuple(fields, variant_ty));
                ValueExpr::Variant { discriminant, data, enum_ty: ty }
            }
            Type::Array { .. } => {
                let mut t: List<ValueExpr> = List::new();
                let mut iter = ecx.project_array_fields(&val).unwrap();
                while let Ok(Some((_, field))) = iter.next(ecx) {
                    let field = self.translate_const_val(field, ecx, span);
                    t.push(field);
                }
                ValueExpr::Tuple(t, ty)
            }
            Type::Union { .. } =>
                rs::span_bug!(span, "Constant Unions are currently not supported!"),
            Type::Slice { .. } => rs::span_bug!(span, "constant slices do not exist!"),
        }
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
                let start = offset.bytes_usize();
                let end = start + DefaultTarget::PTR_SIZE.bytes().try_to_usize().unwrap();
                // Pointer bytes are always initialized, so we can unwrap.
                let inner_offset = bytes[start..end].iter().map(|x| x.unwrap()).collect();
                let inner_offset = DefaultTarget::ENDIANNESS.decode(Unsigned, inner_offset);
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
