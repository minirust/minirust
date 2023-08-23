use super::*;

pub(super) fn fmt_globals(globals: Map<GlobalName, Global>) -> String {
    let mut out = String::new();

    let mut globals: Vec<(GlobalName, Global)> = globals.iter().collect();

    // The globals are formatted in the order of their names.
    globals.sort_by_key(|(GlobalName(name), _global)| *name);

    for (gname, global) in globals {
        out += &fmt_global(gname, global);
    }
    out
}

pub(super) fn fmt_relocation(relocation: Relocation) -> FmtExpr {
    let gname = fmt_global_name(relocation.name);

    if relocation.offset.bytes() == 0 {
        FmtExpr::Atomic(gname)
    } else {
        let offset = relocation.offset.bytes();
        FmtExpr::NonAtomic(format!("{gname} + {offset}"))
    }
}

fn fmt_global(gname: GlobalName, global: Global) -> String {
    let gname_str = fmt_global_name(gname);
    let bytes_str = fmt_bytes(global.bytes);
    let align = global.align.bytes();
    let mut out = format!(
        "{gname_str} {{
  bytes = [{bytes_str}],
  align = {align} bytes,\n"
    );
    for (i, rel) in global.relocations {
        let i = i.bytes();
        let rel_str = fmt_relocation(rel).to_string();
        out += &format!("  at byte {i}: {rel_str},\n");
    }
    out += "}\n\n";
    out
}

fn fmt_bytes(bytes: List<Option<u8>>) -> String {
    let b: Vec<_> = bytes
        .iter()
        .map(|x| match x {
            Some(u) => format!("{:02x?}", u),
            None => format!("__"),
        })
        .collect();

    b.join(" ")
}
