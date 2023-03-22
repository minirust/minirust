use super::*;

pub(super) fn fmt_place_expr(p: PlaceExpr, comptypes: &mut Vec<CompType>) -> String {
    match p {
        PlaceExpr::Local(l) => fmt_local_name(l),
        PlaceExpr::Deref { operand, ptype } => {
            let ptype = fmt_ptype(ptype, comptypes);
            let expr = fmt_value_expr(operand.extract(), comptypes);
            format!("deref<{ptype}>({expr})")
        }
        PlaceExpr::Field { root, field } => {
            let root = fmt_place_expr(root.extract(), comptypes);
            format!("{root}.{field}")
        }
        PlaceExpr::Index { root, index } => {
            let root = fmt_place_expr(root.extract(), comptypes);
            let index = fmt_value_expr(index.extract(), comptypes);
            format!("{root}[{index}]")
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

fn fmt_constant(c: Constant) -> String {
    match c {
        Constant::Int(int) => int.to_string(),
        Constant::Bool(b) => b.to_string(),
        Constant::GlobalPointer(relocation) => fmt_relocation(relocation),
        Constant::FnPointer(fn_name) => fmt_fn_name(fn_name),
        Constant::Variant { .. } => panic!("enums are unsupported!"),
    }
}

pub(super) fn fmt_value_expr(v: ValueExpr, comptypes: &mut Vec<CompType>) -> String {
    match v {
        ValueExpr::Constant(c, _ty) => fmt_constant(c),
        ValueExpr::Tuple(l, t) => {
            let (lparen, rparen) = match t {
                Type::Array { .. } => ('[', ']'),
                Type::Tuple { .. } => ('(', ')'),
                _ => panic!(),
            };
            let l: Vec<_> = l.iter().map(|x| fmt_value_expr(x, comptypes)).collect();
            let l = l.join(", ");

            format!("{lparen}{l}{rparen}")
        }
        ValueExpr::Union {
            field,
            expr,
            union_ty,
        } => {
            let union_ty = fmt_type(union_ty, comptypes);
            let expr = fmt_value_expr(expr.extract(), comptypes);
            format!("{union_ty} {{ field{field}: {expr} }}")
        }
        ValueExpr::Load {
            destructive,
            source,
        } => {
            let source = source.extract();
            let source = fmt_place_expr(source, comptypes);
            let load_name = match destructive {
                true => "move",
                false => "load",
            };
            format!("{load_name}({source})")
        }
        ValueExpr::AddrOf {
            target,
            ptr_ty: PtrType::Raw { .. },
        } => {
            let target = target.extract();
            let target = fmt_place_expr(target, comptypes);
            format!("&raw {target}")
        }
        ValueExpr::AddrOf {
            target,
            ptr_ty: PtrType::Ref { mutbl, .. },
        } => {
            let target = target.extract();
            let target = fmt_place_expr(target, comptypes);
            let mutbl = match mutbl {
                Mutability::Mutable => "mut ",
                Mutability::Immutable => "",
            };
            format!("&{mutbl}{target}")
        }
        ValueExpr::AddrOf {
            target: _,
            ptr_ty: _,
        } => {
            panic!("unsupported ptr_ty for AddrOr!")
        }
        ValueExpr::UnOp { operator, operand } => {
            let operand = fmt_value_expr(operand.extract(), comptypes);
            match operator {
                UnOp::Int(UnOpInt::Neg, _int_ty) => format!("(-{operand})"),
                UnOp::Int(UnOpInt::Cast, _int_ty) => format!("int2int({operand})"),
                UnOp::Ptr2Ptr(_ptr_ty) => format!("ptr2ptr({operand})"),
                UnOp::Ptr2Int => format!("ptr2int({operand})"),
                UnOp::Int2Ptr(_ptr_ty) => format!("int2ptr({operand})"),
            }
        }
        ValueExpr::BinOp {
            operator: BinOp::Int(int_op, int_ty),
            left,
            right,
        } => {
            let int_op = match int_op {
                BinOpInt::Add => '+',
                BinOpInt::Sub => '-',
                BinOpInt::Mul => '*',
                BinOpInt::Div => '/',
                BinOpInt::Rem => '%',
            };

            let int_ty = fmt_int_type(int_ty);
            let int_op = format!("{int_op}<{int_ty}>");

            let l = fmt_value_expr(left.extract(), comptypes);
            let r = fmt_value_expr(right.extract(), comptypes);

            format!("({l} {int_op} {r})")
        }
        ValueExpr::BinOp {
            operator: BinOp::IntRel(rel),
            left,
            right,
        } => {
            let rel = match rel {
                IntRel::Lt => "<",
                IntRel::Le => "<=",
                IntRel::Gt => ">",
                IntRel::Ge => ">=",
                IntRel::Eq => "==",
                IntRel::Ne => "!=",
            };

            let l = fmt_value_expr(left.extract(), comptypes);
            let r = fmt_value_expr(right.extract(), comptypes);

            format!("({l} {rel} {r})")
        }
        ValueExpr::BinOp {
            operator: BinOp::PtrOffset { inbounds },
            left,
            right,
        } => {
            let offset_name = match inbounds {
                true => "offset_inbounds",
                false => "offset_wrapping",
            };
            let l = fmt_value_expr(left.extract(), comptypes);
            let r = fmt_value_expr(right.extract(), comptypes);
            format!("{offset_name}({l}, {r})")
        }
    }
}
