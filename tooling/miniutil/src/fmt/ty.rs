use super::*;

pub(super) fn fmt_ptype(place_ty: PlaceType, comptypes: &mut Vec<CompType>) -> String {
    let ty_str = fmt_type(place_ty.ty, comptypes).to_atomic_string();
    let align = place_ty.align.bytes();
    format!("{ty_str}@align({align})")
}

pub(super) fn fmt_type(t: Type, comptypes: &mut Vec<CompType>) -> FmtExpr {
    match t {
        Type::Int(int_ty) => FmtExpr::Atomic(fmt_int_type(int_ty)),
        Type::Bool => FmtExpr::Atomic(String::from("bool")),
        Type::Ptr(PtrType::Ref {
            mutbl: Mutability::Mutable,
            pointee,
        }) => {
            let layout_str = fmt_layout(pointee);
            FmtExpr::NonAtomic(format!("&mut {layout_str}"))
        }
        Type::Ptr(PtrType::Ref {
            mutbl: Mutability::Immutable,
            pointee,
        }) => {
            let layout_str = fmt_layout(pointee);
            FmtExpr::NonAtomic(format!("&{layout_str}"))
        }
        Type::Ptr(PtrType::Box { pointee }) => {
            let layout_str = fmt_layout(pointee);
            FmtExpr::Atomic(format!("Box<{layout_str}>"))
        }
        Type::Ptr(PtrType::Raw { pointee }) => {
            let layout_str = fmt_layout(pointee);
            FmtExpr::NonAtomic(format!("*{layout_str}"))
        }
        Type::Ptr(PtrType::FnPtr) => FmtExpr::Atomic(String::from("fn()")),
        Type::Tuple { .. } | Type::Union { .. } => {
            let comp_ty = CompType(t);
            let comptype_index = get_comptype_index(comp_ty, comptypes);
            FmtExpr::Atomic(fmt_comptype_index(comptype_index))
        }
        Type::Array { elem, count } => {
            let elem = fmt_type(elem.extract(), comptypes).to_string();
            FmtExpr::Atomic(format!("[{elem}; {count}]"))
        }
        Type::Enum { .. } => panic!("enums are unsupported!"),
    }
}

pub(super) fn fmt_int_type(int_ty: IntType) -> String {
    let signed = match int_ty.signed {
        Signed => "i",
        Unsigned => "u",
    };
    let bits = int_ty.size.bits();

    format!("{signed}{bits}")
}

fn fmt_layout(layout: Layout) -> String {
    let size = layout.size.bytes();
    let align = layout.align.bytes();
    let uninhab_str = match layout.inhabited {
        true => "",
        false => ", uninhabited",
    };
    format!("layout(size={size}, align={align}{uninhab_str})")
}

/////////////////////
// composite types
/////////////////////

// A "composite" type is a union or tuple (enums aren't yet supported).
// Composite types will be printed separately above the functions, as inlining them would be hard to read.
// During formatting, the list of composite types we encounter will be stored in `comptypes`.
#[derive(PartialEq, Eq, Clone, Copy)]
pub(super) struct CompType(pub(super) Type);

// An index into `comptypes`.
pub(super) struct CompTypeIndex {
    idx: usize,
}

// Gives the index of `ty` within `comptypes`.
// This adds `ty` to `comptypes` if it has been missing.
fn get_comptype_index(ty: CompType, comptypes: &mut Vec<CompType>) -> CompTypeIndex {
    let idx = match comptypes.iter().position(|x| *x == ty) {
        Some(i) => i,
        None => {
            let n = comptypes.len();
            comptypes.push(ty);
            n
        }
    };

    CompTypeIndex { idx }
}

fn fmt_comptype_index(comptype_index: CompTypeIndex) -> String {
    let id = comptype_index.idx;
    format!("T{id}")
}

// Formats all composite types.
pub(super) fn fmt_comptypes(mut comptypes: Vec<CompType>) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < comptypes.len() {
        let c = comptypes[i];
        let comptype_index = CompTypeIndex { idx: i };

        // A call to `fmt_comptype` might find new `CompTypes` and push them to `comptypes`.
        // Hence, we cannot use an iterator here.
        let s = &*fmt_comptype(comptype_index, c, &mut comptypes);

        out += s;

        i += 1;
    }

    out
}

fn fmt_comptype(i: CompTypeIndex, t: CompType, comptypes: &mut Vec<CompType>) -> String {
    let (keyword, fields, opt_chunks, size) = match t.0 {
        Type::Tuple { fields, size } => ("tuple", fields, None, size),
        Type::Union {
            chunks,
            fields,
            size,
        } => ("union", fields, Some(chunks), size),
        _ => panic!("not a supported composite type!"),
    };
    let ct = fmt_comptype_index(i).to_string();
    let size = size.bytes();
    let mut s = format!("{keyword} {ct} ({size} bytes) {{\n");
    for (offset, f) in fields {
        let offset = offset.bytes();
        let ty = fmt_type(f, comptypes).to_string();
        s += &format!("  at byte {offset}: {ty},\n");
    }
    if let Some(chunks) = opt_chunks {
        for (offset, size) in chunks {
            let offset = offset.bytes();
            let size = size.bytes();
            s += &format!("  chunk(at={offset}, size={size}),\n");
        }
    }
    s += "}\n\n";
    s
}
