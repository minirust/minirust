//! This module makes it easy to create a `Program`.
#![allow(unused)]

use crate::*;

mod function;
pub use function::*;

mod statement; // also includes terminators
pub use statement::*;

mod expr;
pub use expr::*;

mod ty;
pub use ty::*;

mod ty_conv;
pub use ty_conv::*;

pub fn align(bytes: u32) -> Align {
    Align::from_bytes(bytes).unwrap()
}

pub fn size(bytes: u32) -> Size {
    Size::from_bytes(bytes).unwrap()
}

/// generates a small program, only having one basic block.
pub fn small_program(locals: &[PlaceType], statements: &[Statement]) -> Program {
    let b = block(statements, exit());
    let f = function(Ret::No, 0, locals, &[b]);

    program(&[f])
}
