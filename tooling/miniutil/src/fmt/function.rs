use super::*;

// Formats all functions found within the program.
// All composite types that are used within `prog` will be added to `comptypes` exactly once.
pub(super) fn fmt_functions(prog: Program, comptypes: &mut Vec<CompType>) -> String {
    let mut fns: Vec<(FnName, Function)> = prog.functions.iter().collect();

    // Functions are formatted in the order given by their name.
    fns.sort_by_key(|(FnName(name), _fn)| *name);

    let mut out = String::new();
    for (fn_name, f) in fns {
        let start = prog.start == fn_name;
        out += &fmt_function(fn_name, f, start, comptypes);
    }

    out
}

fn fmt_function(
    fn_name: FnName,
    f: Function,
    start: bool,
    comptypes: &mut Vec<CompType>,
) -> String {
    let fn_name = fmt_fn_name(fn_name);

    // Format function arguments
    let args: Vec<String> = f
        .args
        .iter()
        .map(|(name, _arg_abi)| {
            let ident = fmt_local_name(name);
            let ty = fmt_ptype(f.locals.index_at(name), comptypes);

            format!("{ident}: {ty}")
        })
        .collect();
    let args = args.join(", ");

    // Format return type
    let mut ret_ty = String::from("none");
    if let Some((ret, _arg_abi)) = f.ret {
        ret_ty = fmt_ptype(f.locals.index_at(ret), comptypes);
    }

    // Format function signature
    let mut out = if start {
        format!("start fn {fn_name}({args}) -> {ret_ty} {{\n")
    } else {
        format!("fn {fn_name}({args}) -> {ret_ty} {{\n")
    };

    // Format locals
    let mut locals: Vec<(LocalName, PlaceType)> = f.locals.iter().collect();

    // The locals are formatted in the order of their names.
    locals.sort_by_key(|(LocalName(name), _place_ty)| *name);

    for (l, pty) in locals {
        let local = fmt_local_name(l);
        let ptype = fmt_ptype(pty, comptypes);
        out += &format!("  let {local}: {ptype};\n");
    }

    // Format basic blocks
    let mut blocks: Vec<(BbName, BasicBlock)> = f.blocks.iter().collect();

    // Basic blocks are formatted in the order of their names.
    blocks.sort_by_key(|(BbName(name), _block)| *name);

    for (bb_name, bb) in blocks {
        let start = f.start == bb_name;
        out += &fmt_bb(bb_name, bb, start, comptypes);
    }
    out += "}\n\n";

    out
}

fn fmt_bb(bb_name: BbName, bb: BasicBlock, start: bool, comptypes: &mut Vec<CompType>) -> String {
    let name = bb_name.0.get_internal();

    let mut out = if start {
        format!("  start bb{name}:\n")
    } else {
        format!("  bb{name}:\n")
    };

    // Format statements
    for st in bb.statements.iter() {
        out += &fmt_statement(st, comptypes);
        out.push('\n');
    }
    // Format terminator
    out += &fmt_terminator(bb.terminator, comptypes);
    out.push('\n');
    out
}

fn fmt_statement(st: Statement, comptypes: &mut Vec<CompType>) -> String {
    match st {
        Statement::Assign {
            destination,
            source,
        } => {
            let left = fmt_place_expr(destination, comptypes);
            let right = fmt_value_expr(source, comptypes);
            format!("    {left} = {right};")
        }
        Statement::Finalize { place, fn_entry } => {
            let place = fmt_place_expr(place, comptypes);
            format!("    Finalize({place}, {fn_entry});")
        }
        Statement::StorageLive(local) => {
            let local = fmt_local_name(local);
            format!("    StorageLive({local});")
        }
        Statement::StorageDead(local) => {
            let local = fmt_local_name(local);
            format!("    StorageDead({local});")
        }
    }
}

// used both for functions and intrinsics.
fn fmt_call(
    callee: &str,
    arguments: List<ValueExpr>,
    ret: Option<PlaceExpr>,
    next_block: Option<BbName>,
    comptypes: &mut Vec<CompType>,
) -> String {
    // Format function args
    let args: Vec<_> = arguments
        .iter()
        .map(|x| fmt_value_expr(x, comptypes))
        .collect();
    let args = args.join(", ");

    // Format return place
    let r = match ret {
        Some(ret) => fmt_place_expr(ret, comptypes),
        None => String::from("none"),
    };

    // Format next block
    let next = match next_block {
        Some(next_block) => {
            let next_str = fmt_bb_name(next_block);
            format!(" -> {next_str}")
        }
        None => String::new(),
    };

    format!("    {r} = {callee}({args}){next};")
}

fn fmt_terminator(t: Terminator, comptypes: &mut Vec<CompType>) -> String {
    match t {
        Terminator::Goto(bb) => {
            let bb = fmt_bb_name(bb);
            format!("    goto -> {bb};")
        }
        Terminator::If {
            condition,
            then_block,
            else_block,
        } => {
            let branch_expr = fmt_value_expr(condition, comptypes);
            let then_bb = fmt_bb_name(then_block);
            let else_bb = fmt_bb_name(else_block);
            format!(
                "    if {branch_expr} {{
      goto -> {then_bb};
    }} else {{
      goto -> {else_bb};
    }}"
            )
        }
        Terminator::Unreachable => {
            format!("    unreachable;")
        }
        Terminator::Call {
            callee,
            arguments,
            ret,
            next_block,
        } => {
            let callee = fmt_value_expr(callee, comptypes);
            let arguments = arguments.iter().map(|(expr, _arg_abi)| expr).collect();
            let ret = ret.map(|(place_expr, _arg_abi)| place_expr);
            fmt_call(&callee, arguments, ret, next_block, comptypes)
        }
        Terminator::Return => {
            format!("    return;")
        }
        Terminator::CallIntrinsic {
            intrinsic,
            arguments,
            ret,
            next_block,
        } => {
            let callee = match intrinsic {
                Intrinsic::Exit => "exit",
                Intrinsic::PrintStdout => "print",
                Intrinsic::PrintStderr => "eprint",
                Intrinsic::Allocate => "allocate",
                Intrinsic::Deallocate => "deallocate",
            };
            fmt_call(callee, arguments, ret, next_block, comptypes)
        }
    }
}

fn fmt_bb_name(bb: BbName) -> String {
    let id = bb.0.get_internal();
    format!("bb{id}")
}

pub(super) fn fmt_fn_name(fn_name: FnName) -> String {
    let id = fn_name.0.get_internal();
    format!("f{id}")
}
