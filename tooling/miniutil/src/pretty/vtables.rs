use super::*;

pub(super) fn fmt_vtables(vtables: Map<VTableName, VTable>) -> String {
    let mut out = String::new();

    let mut vtables: Vec<(VTableName, VTable)> = vtables.iter().collect();

    // The vtables are formatted in the order of their names.
    vtables.sort_by_key(|(VTableName(name), _)| *name);

    for (vname, vtable) in vtables {
        out += &fmt_vtable(vname, vtable);
    }
    out += "\n";
    out
}

pub fn fmt_vtable_name(vname: VTableName) -> String {
    format!("vtable{id}", id = vname.0.get_internal())
}

fn fmt_vtable(vname: VTableName, vtable: VTable) -> String {
    let mut out = fmt_vtable_name(vname);
    out += " {\n";

    out += &format!("  trait = {},\n", fmt_trait_name(vtable.trait_name));
    out += &format!("  size = {},\n", vtable.size.bytes());
    out += &format!("  align = {},\n", vtable.align.bytes());

    for (meth, impel) in vtable.methods {
        out += &format!(
            "  {meth}() = {f},\n",
            meth = fmt_trait_method_name(meth),
            f = fmt_fn_name(impel)
        );
    }

    out += "}\n";

    out
}

pub fn fmt_traits(traits: Map<TraitName, Set<TraitMethodName>>) -> String {
    let mut out = String::new();
    let mut traits: Vec<(TraitName, Set<TraitMethodName>)> = traits.iter().collect();
    // The traits are formatted in the order of their names.
    traits.sort_by_key(|(TraitName(name), _)| *name);
    for (trait_, methods) in traits {
        out += &fmt_trait(trait_, methods);
    }
    out += "\n";
    out
}

pub fn fmt_trait_name(trait_name: TraitName) -> String {
    format!("trait{id}", id = trait_name.0.get_internal())
}

fn fmt_trait(trait_name: TraitName, methods: Set<TraitMethodName>) -> String {
    let mut out = fmt_trait_name(trait_name);
    out += " { ";
    let mut methods: Vec<String> = methods.iter().map(fmt_trait_method_name).collect();

    methods.sort();
    out += &methods.join(", ");

    out += " }\n";
    out
}

pub fn fmt_trait_method_name(name: TraitMethodName) -> String {
    format!("m{id}", id = name.0.get_internal())
}
