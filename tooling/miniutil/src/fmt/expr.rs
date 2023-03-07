use super::*;

pub fn place_expr_to_string(p: PlaceExpr, comptypes: &mut Vec<Type>) -> String {
    match p {
        PlaceExpr::Local(l) => local_name_to_string(l),
        PlaceExpr::Deref { operand, ptype } => {
            format!("*{} as {}", value_expr_to_string(operand.get(), comptypes), ptype_to_string(ptype, comptypes))
        },
        PlaceExpr::Field { root, field } => {
            let root = root.get();
            format!("{}.{}", place_expr_to_string(root, comptypes), field)
        },
        PlaceExpr::Index { root, index } => {
            let root = root.get();
            let index = index.get();
            format!("{}[{}]", place_expr_to_string(root, comptypes), value_expr_to_string(index, comptypes))
        },
    }
}

pub fn local_name_to_string(l: LocalName) -> String {
    format!("_{}", l.0.get())
}

pub fn global_name_to_string(g: GlobalName) -> String {
    format!("global({})", g.0.get())
}

fn constant_to_string(c: Constant) -> String {
    match c {
        Constant::Int(int) => int.to_string(),
        Constant::Bool(b) => b.to_string(),
        Constant::GlobalPointer(relocation) => relocation_to_string(relocation),
        Constant::FnPointer(fn_name) => fn_name_to_string(fn_name),
        Constant::Variant { .. } => panic!("enums are unsupported!"),
    }
}

pub fn value_expr_to_string(v: ValueExpr, comptypes: &mut Vec<Type>) -> String {
    match v {
        ValueExpr::Constant(c, _ty) => constant_to_string(c),
        ValueExpr::Tuple(l, t) => {
            let (lparen, rparen) = match t {
                Type::Array { .. } => ('[', ']'),
                Type::Tuple { .. } => ('(', ')'),
                _ => panic!(),
            };
            let l: Vec<_> = l.iter().map(|x| value_expr_to_string(x, comptypes)).collect();
            let l = l.join(", ");

            format!("{lparen}{l}{rparen}")
        },
        ValueExpr::Union { field, expr, union_ty } => {
            let union_ty = type_to_string(union_ty, comptypes);
            let expr = value_expr_to_string(expr.get(), comptypes);
            format!("{union_ty} {{ field{field}: {expr} }}")
        },
        ValueExpr::Load { destructive, source } => {
            let source = source.get();
            let source = place_expr_to_string(source, comptypes);
            let load_name = match destructive {
                true => "move",
                false => "load",
            };
            format!("{load_name}({source})")
        },
        ValueExpr::AddrOf { target, ptr_ty: PtrType::Raw { .. } } => {
            let target = target.get();
            let target = place_expr_to_string(target, comptypes);
            format!("&raw {target}")
        },
        ValueExpr::AddrOf { target, ptr_ty: PtrType::Ref { mutbl, .. } } => {
            let target = target.get();
            let target = place_expr_to_string(target, comptypes);
            let mutbl = match mutbl {
                Mutability::Mutable => "mut ",
                Mutability::Immutable => "",
            };
            format!("&{mutbl}{target}")
        },
        ValueExpr::AddrOf { target: _, ptr_ty: _ } => {
            panic!("unsupported ptr_ty for AddrOr!")
        },
        ValueExpr::UnOp { operator, operand } => {
            let operand = value_expr_to_string(operand.get(), comptypes);
            match operator {
                UnOp::Int(UnOpInt::Neg, _int_ty) => format!("-{}", operand),
                UnOp::Int(UnOpInt::Cast, _int_ty) => format!("{} as _", operand),
                UnOp::Ptr2Ptr(_ptr_ty) => format!("{} as _", operand),
                UnOp::Ptr2Int => format!("{} as _", operand),
                UnOp::Int2Ptr(_ptr_ty) => format!("{} as _", operand),
            }
        },
        ValueExpr::BinOp { operator: BinOp::Int(int_op, int_ty), left, right } => {
            let int_op = match int_op {
                BinOpInt::Add => '+',
                BinOpInt::Sub => '-',
                BinOpInt::Mul => '*',
                BinOpInt::Div => '/',
                BinOpInt::Rem => '%',
            };

            let int_ty = int_type_to_string(int_ty);
            let int_op = format!("{int_op}_{int_ty}");

            let l = value_expr_to_string(left.get(), comptypes);
            let r = value_expr_to_string(right.get(), comptypes);

            format!("{l} {int_op} {r}")
        },
        ValueExpr::BinOp { operator: BinOp::IntRel(rel), left, right } => {
            let rel = match rel {
                IntRel::Lt => "<",
                IntRel::Le => "<=",
                IntRel::Gt => ">",
                IntRel::Ge => ">=",
                IntRel::Eq => "==",
                IntRel::Ne => "!=",
            };

            let l = value_expr_to_string(left.get(), comptypes);
            let r = value_expr_to_string(right.get(), comptypes);

            format!("{l} {rel} {r}")
        },
        ValueExpr::BinOp { operator: BinOp::PtrOffset { inbounds }, left, right } => {
            let offset_name = match inbounds {
                true => "offset_inbounds",
                false => "offset_wrapping",
            };
            let l = value_expr_to_string(left.get(), comptypes);
            let r = value_expr_to_string(right.get(), comptypes);
            format!("{offset_name}({l}, {r})")
        }
    }
}

pub fn bb_name_to_string(bb: BbName) -> String {
    format!("bb{}", bb.0.get())
}

pub fn fn_name_to_string(fn_name: FnName) -> String {
    format!("f{}", fn_name.0.get())
}
