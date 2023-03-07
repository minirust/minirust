use crate::*;

pub fn ptype_to_string(place_ty: PlaceType, comptypes: &mut Vec<Type>) -> String {
    format!("{}<align={}>", type_to_string(place_ty.ty, comptypes), place_ty.align.bytes())
}

use std::fmt::Write;
pub fn int_type_to_string(int_ty: IntType) -> String {
    let signed = match int_ty.signed {
        Signed => "i",
        Unsigned => "u",
    };
    let bits = int_ty.size.bits();

    format!("{signed}{bits}")
}

fn layout_to_string(layout: Layout) -> String {
    let uninhab_str = match layout.inhabited {
        true => "",
        false => ", uninhabited",
    };
    format!("layout(size={}, align={}{})", layout.size.bytes(), layout.align.bytes(), uninhab_str)
}

pub fn type_to_string(t: Type, comptypes: &mut Vec<Type>) -> String {
    match t {
        Type::Int(int_ty) => int_type_to_string(int_ty),
        Type::Bool => String::from("bool"),
        Type::Ptr(PtrType::Ref { mutbl: Mutability::Mutable, pointee }) => format!("&mut {}", layout_to_string(pointee)),
        Type::Ptr(PtrType::Ref { mutbl: Mutability::Immutable, pointee }) => format!("&{}", layout_to_string(pointee)),
        Type::Ptr(PtrType::Box { pointee }) => format!("Box<{}>", layout_to_string(pointee)),
        Type::Ptr(PtrType::Raw { pointee }) => format!("*{}", layout_to_string(pointee)),
        Type::Ptr(PtrType::FnPtr) => String::from("fn()"),
        Type::Tuple { .. } | Type::Union { .. } => {
            let i: usize = match comptypes.iter().position(|x| *x == t) {
                Some(i) => i,
                None => {
                    let n = comptypes.len();
                    comptypes.push(t);
                    n
                }
            };
            format!("T{i}")
        },
        Type::Array { elem, count } => {
            let elem = type_to_string(elem.get(), comptypes);
            format!("[{}; {}]", elem, count)
        },
        Type::Enum { .. } => panic!("enums are unsupported!"),
    }
}

pub fn fmt_comptype(i: usize, t: Type, comptypes: &mut Vec<Type>) -> String {
    let (keyword, fields, opt_chunks, size) = match t {
        Type::Tuple { fields, size } => ("tuple", fields, None, size),
        Type::Union { chunks, fields, size } => ("union", fields, Some(chunks), size),
        _ => panic!("not a supported composite type!"),
    };
    let mut s = String::new();
    writeln!(s, "{} T{} ({} bytes) {{", keyword, i, size.bytes()).unwrap();
    for (offset, f) in fields {
        writeln!(s, "  at byte {}: {},", offset.bytes(), type_to_string(f, comptypes)).unwrap()
    }
    if let Some(chunks) = opt_chunks {
        for (offset, size) in chunks {
            write!(s, "  chunk(at={}, size={}),\n", offset.bytes(), size.bytes()).unwrap()
        }
    }
    writeln!(s, "}}\n").unwrap();
    s
}
