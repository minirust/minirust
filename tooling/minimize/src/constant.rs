use crate::*;

pub fn translate_const<'cx, 'tcx>(
    c: &rs::Constant<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> ValueExpr {
    match c.literal {
        rs::ConstantKind::Ty(_) => panic!("not supported!"),
        rs::ConstantKind::Unevaluated(uneval, ty) => translate_const_uneval(uneval, ty, fcx),
        rs::ConstantKind::Val(val, ty) => translate_const_val(val, ty, fcx),
    }
}

fn translate_const_val<'cx, 'tcx>(
    val: rs::ConstValue<'tcx>,
    ty: rs::Ty<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> ValueExpr {
    let ty = translate_ty(ty, fcx.cx.tcx);

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
            let (alloc_id, offset) = val
                .try_to_scalar()
                .unwrap()
                .to_pointer(&fcx.cx.tcx)
                .unwrap()
                .into_parts();
            let alloc_id = alloc_id.expect("no alloc id?");
            let rel = translate_relocation(alloc_id, offset, fcx);
            Constant::GlobalPointer(rel)
        }
        ty => panic!("unsupported type for `ConstVal`: {:?}", ty),
    };
    ValueExpr::Constant(constant, ty)
}

fn translate_const_uneval<'cx, 'tcx>(
    uneval: rs::UnevaluatedConst<'tcx>,
    ty: rs::Ty<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> ValueExpr {
    let Ok(Some(instance)) = rs::Instance::resolve(fcx.cx.tcx, rs::ParamEnv::empty(), uneval.def, uneval.substs) else {
        panic!("can't resolve unevaluated const!")
    };
    let cid = rs::GlobalId {
        instance,
        promoted: uneval.promoted,
    };
    let alloc = fcx
        .cx
        .tcx
        .eval_to_allocation_raw(rs::ParamEnv::empty().with_const().and(cid))
        .unwrap();
    let name = translate_alloc_id(alloc.alloc_id, fcx);
    let offset = Size::ZERO;

    let rel = Relocation { name, offset };
    relocation_to_value_expr(rel, ty, fcx)
}

fn relocation_to_value_expr<'cx, 'tcx>(
    rel: Relocation,
    ty: rs::Ty<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> ValueExpr {
    let expr = Constant::GlobalPointer(rel);

    let pty = place_type_of(ty, fcx);
    let ptr_ty = Type::Ptr(PtrType::Raw);

    let expr = ValueExpr::Constant(expr, ptr_ty);
    let expr = PlaceExpr::Deref {
        operand: GcCow::new(expr),
        ptype: pty,
    };
    ValueExpr::Load {
        source: GcCow::new(expr),
    }
}

fn translate_relocation<'cx, 'tcx>(
    alloc_id: rs::AllocId,
    offset: rs::Size,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> Relocation {
    let name = translate_alloc_id(alloc_id, fcx);
    let offset = translate_size(offset);
    Relocation { name, offset }
}

// calls `translate_const_allocation` with the allocation of alloc_id,
// and adds the alloc_id and its newly-created global to alloc_map.
fn translate_alloc_id<'cx, 'tcx>(alloc_id: rs::AllocId, fcx: &mut FnCtxt<'cx, 'tcx>) -> GlobalName {
    if let Some(x) = fcx.cx.alloc_map.get(&alloc_id) {
        return *x;
    }

    let name = fresh_global_name(fcx);
    fcx.cx.alloc_map.insert(alloc_id, name);

    let alloc = match fcx.cx.tcx.global_alloc(alloc_id) {
        rs::GlobalAlloc::Memory(alloc) => alloc,
        rs::GlobalAlloc::Static(def_id) => fcx.cx.tcx.eval_static_initializer(def_id).unwrap(),
        _ => panic!("unsupported!"),
    };
    translate_const_allocation(alloc, fcx, name);
    name
}

// adds a Global representing this ConstAllocation, and returns the corresponding GlobalName.
fn translate_const_allocation<'cx, 'tcx>(
    allocation: rs::ConstAllocation<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
    name: GlobalName,
) {
    let allocation = allocation.inner();
    let size = allocation.size();
    let alloc_range = rs::AllocRange {
        start: rs::Size::ZERO,
        size,
    };
    let mut bytes: Vec<Option<u8>> = allocation
        .get_bytes_unchecked(alloc_range)
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
                [..BasicMemory::PTR_SIZE.bytes().try_to_usize().unwrap()];
            let inner_offset_bytes: List<u8> =
                inner_offset_bytes.iter().map(|x| x.unwrap()).collect();
            let inner_offset: Int = BasicMemory::ENDIANNESS.decode(Unsigned, inner_offset_bytes);
            let inner_offset = rs::Size::from_bytes(inner_offset.try_to_usize().unwrap());
            let relo = translate_relocation(alloc_id, inner_offset, fcx);

            let offset = translate_size(offset);
            (offset, relo)
        })
        .collect();
    let align = translate_align(allocation.align);
    let global = Global {
        bytes: bytes.into_iter().collect(),
        relocations,
        align,
    };

    fcx.cx.globals.insert(name, global);
}

fn fresh_global_name<'cx, 'tcx>(fcx: &mut FnCtxt<'cx, 'tcx>) -> GlobalName {
    let name = GlobalName(Name::from_internal(fcx.cx.globals.iter().count() as _)); // TODO use .len() here, if supported
                                                                                    // the default_global is added so that calling `fresh_global_name` twice returns different names.
    let default_global = Global {
        bytes: Default::default(),
        relocations: Default::default(),
        align: Align::ONE,
    };
    fcx.cx.globals.insert(name, default_global);
    name
}
