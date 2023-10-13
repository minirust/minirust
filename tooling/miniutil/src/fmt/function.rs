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
    let fn_name = fmt_fn_name(fn_name).to_string();

    // Format function arguments
    let args: Vec<String> = f
        .args
        .iter()
        .map(|name| fmt_local_name(name).to_string())
        .collect();
    let args = args.join(", ");

    // Format return local
    let ret_str = format!("-> {}", fmt_local_name(f.ret));

    // Format function signature
    let mut out = if start {
        format!("start fn {fn_name}({args}) {ret_str} {{\n")
    } else {
        format!("fn {fn_name}({args}) {ret_str} {{\n")
    };

    // Format locals
    let mut locals: Vec<(LocalName, Type)> = f.locals.iter().collect();

    // The locals are formatted in the order of their names.
    locals.sort_by_key(|(LocalName(name), _ty)| *name);

    for (l, ty) in locals {
        let local = fmt_local_name(l).to_string();
        let ty = fmt_type(ty, comptypes).to_string();
        out += &format!("  let {local}: {ty};\n");
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
            let left = fmt_place_expr(destination, comptypes).to_string();
            let right = fmt_value_expr(source, comptypes).to_string();
            format!("    {left} = {right};")
        }
        Statement::Expose { value } => {
            let val = fmt_value_expr(value, comptypes).to_string();
            format!("    expose({val});")
        }
        Statement::Validate { place, fn_entry } => {
            let place = fmt_place_expr(place, comptypes).to_string();
            format!("    validate({place}, {fn_entry});")
        }
        Statement::Deinit { place } => {
            let place = fmt_place_expr(place, comptypes).to_string();
            format!("    deinit({place});")
        }
        Statement::StorageLive(local) => {
            let local = fmt_local_name(local).to_string();
            format!("    storage_live({local});")
        }
        Statement::StorageDead(local) => {
            let local = fmt_local_name(local).to_string();
            format!("    storage_dead({local});")
        }
    }
}

// used both for functions and intrinsics.
fn fmt_call(
    callee: &str,
    args: String,
    ret: PlaceExpr,
    next_block: Option<BbName>,
    comptypes: &mut Vec<CompType>,
) -> String {
    // Format return place
    let r = fmt_place_expr(ret, comptypes).to_string();

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
            let branch_expr = fmt_value_expr(condition, comptypes).to_string();
            let then_bb = fmt_bb_name(then_block).to_string();
            let else_bb = fmt_bb_name(else_block).to_string();
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
            let callee = fmt_value_expr(callee, comptypes).to_atomic_string();
            let args: Vec<_> = arguments
                .iter()
                .map(|arg| match arg {
                    ArgumentExpr::ByValue(value) =>
                        format!("by-value({})", fmt_value_expr(value, comptypes).to_string()),
                    ArgumentExpr::InPlace(place) =>
                        format!("in-place({})", fmt_place_expr(place, comptypes).to_string()),
                })
                .collect();
            fmt_call(&callee, args.join(", "), ret, next_block, comptypes)
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
                Intrinsic::Spawn => "spawn",
                Intrinsic::Join => "join",
                Intrinsic::AtomicStore => "atomic_store",
                Intrinsic::AtomicLoad => "atomic_load",
                Intrinsic::AtomicCompareExchange => "atomic_compare_exchange",
                Intrinsic::AtomicFetchAndOp(binop) => fmt_fetch(binop),
                Intrinsic::Lock(LockIntrinsic::Acquire) => "lock_acquire",
                Intrinsic::Lock(LockIntrinsic::Create) => "lock_create",
                Intrinsic::Lock(LockIntrinsic::Release) => "lock_release",
            };
            let args: Vec<_> = arguments
                .iter()
                .map(|arg| fmt_value_expr(arg, comptypes).to_string())
                .collect();
            fmt_call(callee, args.join(", "), ret, next_block, comptypes)
        }
    }
}

fn fmt_fetch(binop: BinOpInt) -> &'static str {
    use BinOpInt as B;
    match binop {
        B::Add => "atomic_fetch_add",
        B::Sub => "atomic_fetch_sub",
        _ => "atomic_fetch_ILL_FORMED",
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
