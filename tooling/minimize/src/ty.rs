use crate::*;

impl<'tcx> Ctxt<'tcx> {
    pub fn layout_of(&self, ty: rs::Ty<'tcx>) -> Layout {
        let layout = self.rs_layout_of(ty);
        assert!(layout.is_sized(), "encountered unsized type: {ty}");
        let size = translate_size(layout.size());
        let align = translate_align(layout.align().abi);
        let inhabited = !layout.abi().is_uninhabited();

        Layout { size, align, inhabited }
    }

    pub fn layout_of_smir(&self, ty: smir::Ty) -> Layout {
        self.layout_of(smir::internal(self.tcx, ty))
    }

    pub fn translate_ty_smir(&self, ty: smir::Ty, span: rs::Span) -> Type {
        self.translate_ty(smir::internal(self.tcx, ty), span)
    }

    pub fn translate_ty(&self, ty: rs::Ty<'tcx>, span: rs::Span) -> Type {
        match ty.kind() {
            rs::TyKind::Bool => Type::Bool,
            rs::TyKind::Int(int_ty) => Type::Int(translate_int_ty(int_ty)),
            rs::TyKind::Uint(uint_ty) => Type::Int(translate_uint_ty(uint_ty)),
            rs::TyKind::Tuple(ts) => {
                let layout = self.rs_layout_of(ty);
                let size = translate_size(layout.size());
                let align = translate_align(layout.align().abi);

                let fields = ts
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let t = self.translate_ty(t, span);
                        let offset = layout.fields().offset(i);
                        let offset = translate_size(offset);

                        (offset, t)
                    })
                    .collect();

                Type::Tuple { fields, size, align }
            }
            rs::TyKind::Adt(adt_def, sref) if adt_def.is_struct() => {
                let (fields, size, align) = self.translate_adt_fields(ty, *adt_def, sref, span);

                Type::Tuple { fields, size, align }
            }
            rs::TyKind::Adt(adt_def, sref) if adt_def.is_union() => {
                let (fields, size, align) = self.translate_adt_fields(ty, *adt_def, sref, span);
                let chunks = calc_chunks(fields, size);

                Type::Union { fields, size, align, chunks }
            }
            rs::TyKind::Adt(adt_def, sref) if adt_def.is_enum() =>
                self.translate_enum(ty, *adt_def, sref, span),
            rs::TyKind::Adt(adt_def, _) if adt_def.is_box() => {
                let ty = ty.boxed_ty();
                let pointee = self.layout_of(ty);
                Type::Ptr(PtrType::Box { pointee })
            }
            rs::TyKind::Ref(_, ty, mutbl) => {
                let pointee = self.layout_of(*ty);
                let mutbl = translate_mutbl(*mutbl);
                Type::Ptr(PtrType::Ref { pointee, mutbl })
            }
            rs::TyKind::RawPtr(rs::TypeAndMut { ty, mutbl: _ }) => {
                let _pointee = self.layout_of(*ty); // just to make sure that we can translate this type
                Type::Ptr(PtrType::Raw)
            }
            rs::TyKind::Array(ty, c) => {
                let count = Int::from(c.eval_target_usize(self.tcx, rs::ParamEnv::reveal_all()));
                let elem = GcCow::new(self.translate_ty(*ty, span));
                Type::Array { elem, count }
            }
            rs::TyKind::FnPtr(sig) => {
                let abi = self
                    .tcx
                    .fn_abi_of_fn_ptr(rs::ParamEnv::reveal_all().and((*sig, rs::List::empty())))
                    .unwrap();

                Type::Ptr(PtrType::FnPtr(translate_calling_convention(abi.conv)))
            }
            rs::TyKind::Never =>
                build::enum_ty::<u8>(&[], Discriminator::Invalid, build::size(0), build::align(1)),
            x => rs::span_bug!(span, "TyKind not supported: {x:?}"),
        }
    }

    fn translate_adt_fields(
        &self,
        ty: rs::Ty<'tcx>,
        adt_def: rs::AdtDef<'tcx>,
        sref: rs::GenericArgsRef<'tcx>,
        span: rs::Span,
    ) -> (Fields, Size, Align) {
        let layout = self.rs_layout_of(ty);
        let fields = adt_def
            .all_fields()
            .enumerate()
            .map(|(i, field)| {
                let ty = field.ty(self.tcx, sref);
                let ty = self.translate_ty(ty, span);
                let offset = layout.fields().offset(i);
                let offset = translate_size(offset);

                (offset, ty)
            })
            .collect();
        let size = translate_size(layout.size());
        let align = translate_align(layout.align().abi);

        (fields, size, align)
    }
}

pub fn translate_mutbl(mutbl: rs::Mutability) -> Mutability {
    match mutbl {
        rs::Mutability::Mut => Mutability::Mutable,
        rs::Mutability::Not => Mutability::Immutable,
    }
}

pub fn translate_mutbl_smir(mutbl: smir::Mutability) -> Mutability {
    match mutbl {
        smir::Mutability::Mut => Mutability::Mutable,
        smir::Mutability::Not => Mutability::Immutable,
    }
}

fn translate_int_ty(int_ty: &rs::IntTy) -> IntType {
    use rs::IntTy::*;

    let size = match int_ty {
        Isize => 8, // this is fixed as 8, to be compatible with BasicMemory.
        I8 => 1,
        I16 => 2,
        I32 => 4,
        I64 => 8,
        I128 => 16,
    };

    let signed = Signedness::Signed;
    let size = Size::from_bytes_const(size);
    IntType { signed, size }
}

fn translate_uint_ty(uint_ty: &rs::UintTy) -> IntType {
    use rs::UintTy::*;

    let size = match uint_ty {
        Usize => 8, // this is fixed as 8, to be compatible with BasicMemory.
        U8 => 1,
        U16 => 2,
        U32 => 4,
        U64 => 8,
        U128 => 16,
    };

    let signed = Signedness::Unsigned;
    let size = Size::from_bytes_const(size);
    IntType { signed, size }
}

pub fn translate_size(size: rs::Size) -> Size {
    Size::from_bytes_const(size.bytes())
}

pub fn translate_align(align: rs::Align) -> Align {
    Align::from_bytes(align.bytes()).unwrap()
}

pub fn translate_calling_convention(conv: rs::Conv) -> CallingConvention {
    match conv {
        rs::Conv::C => CallingConvention::C,
        rs::Conv::Rust => CallingConvention::Rust,
        _ => todo!(),
    }
}
