use super::*;

// A formatted expression.
// This type is used to insert the minimal required amount of parens to make expressions unambiguous, without implementing an operator priority.
pub(super) enum FmtExpr {
    // An expression that might be ambiguous in certain contexts.
    // For example `a + b` is ambiguous in the context `a + b * c`: It might mean `(a + b) * c` or `a + (b * c)`
    NonAtomic(String),

    // An expression that is never ambiguous.
    // For example `2`, `(a + b)` or `load(_1)`.
    Atomic(String),
}

impl FmtExpr {
    // Returns the contents of this FmtExpr as-is, without wrapping in `(`, `)`.
    // Use this function in unambiguous contexts like `foo(_)`.
    pub(super) fn to_string(self) -> String {
        match self {
            FmtExpr::NonAtomic(s) => s,
            FmtExpr::Atomic(s) => s,
        }
    }

    // Wraps the expression in `(`, `)` if necessary.
    // Use this function in ambiguous contexts like `a + _`.
    pub(super) fn to_atomic_string(self) -> String {
        match self {
            // This adds parens around non-atomic expressions to make them atomic.
            FmtExpr::NonAtomic(s) => format!("({s})"),
            FmtExpr::Atomic(s) => s,
        }
    }
}

pub(super) fn fmt_place_expr(p: PlaceExpr, comptypes: &mut Vec<CompType>) -> FmtExpr {
    match p {
        PlaceExpr::Local(l) => FmtExpr::Atomic(fmt_local_name(l)),
        PlaceExpr::Deref { operand, ty } => {
            let ptype = fmt_type(ty, comptypes).to_string();
            let expr = fmt_value_expr(operand.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("deref<{ptype}>({expr})"))
        }
        PlaceExpr::Field { root, field } => {
            let root = fmt_place_expr(root.extract(), comptypes).to_atomic_string();
            // `&raw foo.bar` in Rust unambiguously means `&raw (foo.bar)`, and there is
            // no other context we have to worry about. Hence this can be atomic.
            FmtExpr::Atomic(format!("{root}.{field}"))
        }
        PlaceExpr::Index { root, index } => {
            let root = fmt_place_expr(root.extract(), comptypes).to_atomic_string();
            let index = fmt_value_expr(index.extract(), comptypes).to_string();
            // This can be considered atomic due to the same reasoning as for PlaceExpr::Field, see above.
            FmtExpr::Atomic(format!("{root}[{index}]"))
        }
        PlaceExpr::Downcast { root, discriminant } => {
            let root = fmt_place_expr(root.extract(), comptypes).to_atomic_string();
            // This is not atomic as `local(1) as variant 3.0` illustrates. (Field 0 of downcast)
            // We can't do it nicely like in the Rust MIR ({root} as {variant name}) since we have no variant names.
            FmtExpr::NonAtomic(format!("{root} as variant {discriminant}"))
        }
    }
}

pub(super) fn fmt_local_name(l: LocalName) -> String {
    let id = l.0.get_internal();
    format!("_{id}")
}

pub(super) fn fmt_global_name(g: GlobalName) -> String {
    let id = g.0.get_internal();
    format!("global({id})")
}

pub(super) fn fmt_constant(c: Constant) -> FmtExpr {
    match c {
        Constant::Int(int) => FmtExpr::Atomic(int.to_string()),
        Constant::Bool(b) => FmtExpr::Atomic(b.to_string()),
        Constant::GlobalPointer(relocation) => fmt_relocation(relocation),
        Constant::FnPointer(fn_name) => FmtExpr::Atomic(fmt_fn_name(fn_name)),
        Constant::VTablePointer(vt_name) =>
            FmtExpr::Atomic(format!("vt{id}", id = vt_name.0.get_internal())),
        Constant::PointerWithoutProvenance(addr) =>
            if addr == 0 {
                FmtExpr::Atomic(format!("nullptr"))
            } else {
                FmtExpr::Atomic(format!("invalid_ptr({addr})"))
            },
    }
}

pub(super) fn fmt_value_expr(v: ValueExpr, comptypes: &mut Vec<CompType>) -> FmtExpr {
    match v {
        ValueExpr::Constant(c, _ty) => fmt_constant(c),
        ValueExpr::Tuple(l, t) => {
            let (lparen, rparen) = match t {
                Type::Array { .. } => ('[', ']'),
                Type::Tuple { .. } => ('(', ')'),
                _ => panic!(),
            };
            let l: Vec<_> = l.iter().map(|x| fmt_value_expr(x, comptypes).to_string()).collect();
            let l = l.join(", ");

            FmtExpr::Atomic(format!("{lparen}{l}{rparen}"))
        }
        ValueExpr::Union { field, expr, union_ty } => {
            let union_ty = fmt_type(union_ty, comptypes).to_string();
            let expr = fmt_value_expr(expr.extract(), comptypes).to_string();
            FmtExpr::NonAtomic(format!("{union_ty} {{ field{field}: {expr} }}"))
        }
        ValueExpr::Variant { discriminant, data, enum_ty } => {
            let enum_ty = fmt_type(enum_ty, comptypes).to_string();
            let expr = fmt_value_expr(data.extract(), comptypes).to_string();
            FmtExpr::NonAtomic(format!("{enum_ty}(variant {discriminant}): {expr}"))
        }
        ValueExpr::GetDiscriminant { place } => {
            let place = fmt_place_expr(place.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("discriminant({place})"))
        }
        ValueExpr::Load { source } => {
            let source = source.extract();
            let source = fmt_place_expr(source, comptypes).to_string();
            FmtExpr::Atomic(format!("load({source})"))
        }
        ValueExpr::AddrOf { target, ptr_ty: PtrType::Raw { .. } } => {
            let target = target.extract();
            let target = fmt_place_expr(target, comptypes).to_atomic_string();
            FmtExpr::NonAtomic(format!("&raw {target}"))
        }
        ValueExpr::AddrOf { target, ptr_ty: PtrType::Ref { mutbl, .. } } => {
            let target = target.extract();
            let target = fmt_place_expr(target, comptypes).to_atomic_string();
            let mutbl = match mutbl {
                Mutability::Mutable => "mut ",
                Mutability::Immutable => "",
            };
            FmtExpr::NonAtomic(format!("&{mutbl}{target}"))
        }
        ValueExpr::AddrOf { target: _, ptr_ty: _ } => {
            panic!("unsupported ptr_ty for AddrOr!")
        }
        ValueExpr::UnOp { operator, operand } => {
            let operand = fmt_value_expr(operand.extract(), comptypes).to_string();
            match operator {
                UnOp::Int(IntUnOp::Neg) => FmtExpr::NonAtomic(format!("-({operand})")),
                UnOp::Int(IntUnOp::BitNot) => FmtExpr::NonAtomic(format!("!({operand}")),
                UnOp::Int(IntUnOp::CountOnes) =>
                    FmtExpr::NonAtomic(format!("count_ones({operand}")),
                UnOp::Cast(CastOp::IntToInt(int_ty)) => {
                    let int_ty = fmt_int_type(int_ty);
                    FmtExpr::Atomic(format!("int2int<{int_ty}>({operand})"))
                }
                UnOp::Cast(CastOp::Transmute(new_ty)) => {
                    let new_ty = fmt_type(new_ty, comptypes).to_string();
                    FmtExpr::Atomic(format!("transmute<{new_ty}>({operand})"))
                }
                UnOp::GetThinPointer => FmtExpr::Atomic(format!("get_thin_ptr({operand})")),
                UnOp::GetMetadata => FmtExpr::Atomic(format!("get_metadata({operand})")),
                UnOp::ComputeSize(ty) => {
                    let ty_str = fmt_type(ty, comptypes).to_string();
                    FmtExpr::Atomic(format!("compute_size<{ty_str}>({operand})"))
                }
                UnOp::ComputeAlign(ty) => {
                    let ty_str = fmt_type(ty, comptypes).to_string();
                    FmtExpr::Atomic(format!("compute_align<{ty_str}>({operand})"))
                }
                UnOp::VTableMethodLookup(method) =>
                    FmtExpr::NonAtomic(format!(
                        "vtable_lookup<m{m_id}>({operand})",
                        m_id = method.0.get_internal()
                    )),
            }
        }
        ValueExpr::BinOp { operator: BinOp::Int(int_op), left, right } => {
            let l = fmt_value_expr(left.extract(), comptypes).to_atomic_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_atomic_string();

            use IntBinOp::*;
            let int_op = match int_op {
                Add => "+",
                Sub => "-",
                Mul => "*",
                Div => "/",
                Rem => "%",
                Shl => "<<",
                Shr => ">>",
                BitAnd => "&",
                BitOr => "|",
                BitXor => "^",
                AddUnchecked => return FmtExpr::Atomic(format!("AddUnchecked({l}, {r})")),
                SubUnchecked => return FmtExpr::Atomic(format!("SubUnchecked({l}, {r})")),
                MulUnchecked => return FmtExpr::Atomic(format!("MulUnchecked({l}, {r})")),
                DivExact => return FmtExpr::Atomic(format!("DivExact({l}, {r})")),
                ShlUnchecked => return FmtExpr::Atomic(format!("ShlUnchecked({l}, {r})")),
                ShrUnchecked => return FmtExpr::Atomic(format!("ShrUnchecked({l}, {r})")),
            };

            FmtExpr::NonAtomic(format!("{l} {int_op} {r}"))
        }
        ValueExpr::BinOp { operator: BinOp::IntWithOverflow(op), left, right } => {
            let l = fmt_value_expr(left.extract(), comptypes).to_atomic_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_atomic_string();

            let name = match op {
                IntBinOpWithOverflow::Add => "Add",
                IntBinOpWithOverflow::Sub => "Sub",
                IntBinOpWithOverflow::Mul => "Mul",
            };
            FmtExpr::Atomic(format!("{name}WithOverflow({l}, {r})"))
        }
        ValueExpr::BinOp { operator: BinOp::Rel(rel), left, right } => {
            let rel = match rel {
                RelOp::Lt => "<",
                RelOp::Le => "<=",
                RelOp::Gt => ">",
                RelOp::Ge => ">=",
                RelOp::Eq => "==",
                RelOp::Ne => "!=",
                RelOp::Cmp => "<=>",
            };

            let l = fmt_value_expr(left.extract(), comptypes).to_atomic_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_atomic_string();

            FmtExpr::NonAtomic(format!("{l} {rel} {r}"))
        }
        ValueExpr::BinOp { operator: BinOp::PtrOffset { inbounds }, left, right } => {
            let offset_name = match inbounds {
                true => "offset_inbounds",
                false => "offset_wrapping",
            };
            let l = fmt_value_expr(left.extract(), comptypes).to_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("{offset_name}({l}, {r})"))
        }
        ValueExpr::BinOp { operator: BinOp::PtrOffsetFrom { inbounds, nonneg }, left, right } => {
            let offset_name = format!(
                "offset_from_{}{}",
                if inbounds { "inbounds" } else { "wrapping" },
                if nonneg { "_nonneg" } else { "" },
            );
            let l = fmt_value_expr(left.extract(), comptypes).to_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("{offset_name}({l}, {r})"))
        }
        ValueExpr::BinOp { operator: BinOp::ConstructWidePointer(ptr_ty), left, right } => {
            let l = fmt_value_expr(left.extract(), comptypes).to_string();
            let r = fmt_value_expr(right.extract(), comptypes).to_string();
            let ptr_ty_str = fmt_ptr_type(ptr_ty).to_string();
            FmtExpr::Atomic(format!("construct_ptr<{ptr_ty_str}>({l}, {r})"))
        }
    }
}
