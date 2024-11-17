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

fn fmt_vtable(vname: VTableName, _vtable: VTable) -> String {
    // TODO(UnsizedTypes): list all methods
    format!("vt{id} TODO\n\n", id = vname.0.get_internal())
}
