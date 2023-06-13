//! This module makes it easy to create a `Program`.
//!
//! Example:
//!
//! ```rust
//! // Our main function has one local of type `usize`.
//! let locals = &[<usize>::get_ptype()];
//!
//! // the basic block `bb` allocates space for this local, and then terminates the program.
//! let bb = block!(storage_live(0), exit());
//!
//! // the function `f` is our main function, it does never return and has no function arguments.
//! let f = function(Ret::No, 0, locals, &[bb]);
//!
//! // Our program only consists of the function `f`.
//! let program = program(&[f]);
//! ```

use crate::*;

mod function;
pub use function::*;

mod global;
pub use global::*;

mod statement; // Also includes terminators
pub use statement::*;

mod expr;
pub use expr::*;

mod ty;
pub use ty::*;

mod ty_conv;
pub use ty_conv::*;

pub fn align(bytes: impl Into<Int>) -> Align {
    let bytes = bytes.into();
    Align::from_bytes(bytes).unwrap()
}

pub fn size(bytes: impl Into<Int>) -> Size {
    Size::from_bytes(bytes).unwrap()
}

// The first function in `fns` is the start function of the program.
pub fn program_with_globals(fns: &[Function], globals: &[Global]) -> Program {
    let functions: Map<FnName, Function> = fns
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let name = FnName(Name::from_internal(i as _));
            (name, *f)
        })
        .collect();

    let globals: Map<GlobalName, Global> = globals
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let name = GlobalName(Name::from_internal(i as _));
            (name, *g)
        })
        .collect();

    Program {
        functions,
        start: FnName(Name::from_internal(0)),
        globals,
    }
}

// The first function in `fns` is the start function of the program.
pub fn program(fns: &[Function]) -> Program {
    program_with_globals(fns, &[])
}

// Generates a small program with a single basic block.
pub fn small_program(locals: &[PlaceType], statements: &[Statement]) -> Program {
    let b = block(statements, exit());
    let f = function(Ret::No, 0, locals, &[b]);

    program(&[f])
}
