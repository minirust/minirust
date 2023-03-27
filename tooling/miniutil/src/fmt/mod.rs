use crate::*;

mod expr;
use expr::*;

mod function;
use function::*;

mod ty;
use ty::*;

mod global;
use global::*;

// Print a program to stdout.
pub fn dump_program(prog: Program) {
    let s = fmt_program(prog);
    println!("{s}");
}

// Format a program into a string.
pub fn fmt_program(prog: Program) -> String {
    let mut comptypes: Vec<CompType> = Vec::new();

    let functions_string = fmt_functions(prog, &mut comptypes);
    let comptypes_string = fmt_comptypes(comptypes);
    let globals_string = fmt_globals(prog.globals);

    comptypes_string + &functions_string + &globals_string
}
