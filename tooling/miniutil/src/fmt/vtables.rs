use super::*;

pub(super) fn fmt_vtables(vtables: Map<VTableName, VTable>) -> String {
    let mut out = String::new();

    let mut vtables: Vec<(VTableName, VTable)> = vtables.iter().collect();

    // The vtables are formatted in the order of their names.
    vtables.sort_by_key(|(VTableName(name), _)| *name);

    for (vname, vtable) in vtables {
        out += &fmt_vtable(vname, vtable);
    }
    out
}

fn fmt_vtable(vname: VTableName, vtable: VTable) -> String {
    let mut out = format!("vtable{id} {{\n", id = vname.0.get_internal());

    out += &format!("  trait = trait{},\n", vtable.trait_name.0.get_internal());
    out += &format!("  size = {},\n", vtable.size.bytes());
    out += &format!("  align = {},\n", vtable.align.bytes());

    for (meth, impel) in vtable.methods {
        out +=
            &format!("  m{m_id}() = {f},\n", m_id = meth.0.get_internal(), f = fmt_fn_name(impel));
    }

    out += "}\n";

    out
}
