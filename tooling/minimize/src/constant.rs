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
        Type::Enum { size, discriminator, variants, .. } => {
            let Some(bits) = val.try_to_bits(rs::Size::from_bits(size.bits().try_to_u8().unwrap())) else {
                panic!("Can only create constant enum from bits. Got {:?}", val);
            };

            // special case for bits = 0 which can be indicative of a None constant.
            // FIXME: Allow for other constant values by using the offset and value type.
            //        However this is probably going to require machine information for
            //        the value encoding which we don't have here.
            if bits == 0 {
                let discriminant = match discriminator {
                    Discriminator::Known(discriminant) => discriminant,
                    Discriminator::Branch { fallback, children, .. } => {
                        let child = children.into_iter()
                                    .find_map(|((start, end), d)| if start <= Int::ZERO && Int::ZERO <= end { Some(d) } else { None })
                                    .unwrap_or(fallback.extract());
                        let Discriminator::Known(discriminant) = child else {
                            unreachable!("Enums minimized from Rust have no nested or invalid discriminators in children.");
                        };
                        discriminant
                    },
                    Discriminator::Invalid => {
                        panic!("Trying to build constant uninhabited enum.")
                    }
                };

                let variant = variants.get(discriminant).unwrap();
                match variant.ty {
                    Type::Tuple { fields, .. } if fields.is_empty() =>
                        return ValueExpr::Variant { discriminant, data: GcCow::new(ValueExpr::Tuple(List::new(), variant.ty)), enum_ty: ty },
                    _ => panic!("Unsupported constant enum variant {:?} with data.", variant),
                }
            } else {
                panic!("Unsupported constant enum with non-zero bits.")
            }
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
    let Ok(Some(instance)) = rs::Instance::resolve(fcx.cx.tcx, rs::ParamEnv::reveal_all(), uneval.def, uneval.args) else {
        panic!("can't resolve unevaluated const!")
    };
    let cid = rs::GlobalId {
        instance,
        promoted: uneval.promoted,
    };
    let alloc = fcx
        .cx
        .tcx
        .eval_to_allocation_raw(rs::ParamEnv::reveal_all().and(cid))
        .unwrap();
    let name = translate_alloc_id(alloc.alloc_id, fcx);
    let offset = Offset::ZERO;

    let rel = Relocation { name, offset };
    relocation_to_value_expr(rel, ty, fcx)
}

fn relocation_to_value_expr<'cx, 'tcx>(
    rel: Relocation,
    ty: rs::Ty<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> ValueExpr {
    let expr = Constant::GlobalPointer(rel);

    let ty = translate_ty(ty, fcx.cx.tcx);
    let ptr_ty = Type::Ptr(PtrType::Raw);

    let expr = ValueExpr::Constant(expr, ptr_ty);
    let expr = PlaceExpr::Deref {
        operand: GcCow::new(expr),
        ty,
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
            let inner_offset: Int = DefaultTarget::ENDIANNESS.decode(Unsigned, inner_offset_bytes);
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
