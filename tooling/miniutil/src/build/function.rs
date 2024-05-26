use crate::build::*;

pub fn fn_ptr(fn_name: u32) -> ValueExpr {
    // For now we use the C ABI for everything since that's what `spawn` needs...
    fn_ptr_conv(fn_name, CallingConvention::C)
}

pub fn fn_ptr_by_name(name: FnName) -> ValueExpr {
    let c = Constant::FnPointer(name);
    // For now we use the C ABI for everything since that's what `spawn` needs...
    let t = Type::Ptr(PtrType::FnPtr(CallingConvention::C));
    ValueExpr::Constant(c, t)
}

pub fn fn_ptr_conv(fn_name: u32, conv: CallingConvention) -> ValueExpr {
    let x = Name::from_internal(fn_name as _);
    let x = FnName(x);
    let x = Constant::FnPointer(x);
    ValueExpr::Constant(x, Type::Ptr(PtrType::FnPtr(conv)))
}

// Whether a function returns or not.
pub enum Ret {
    Yes,
    No,
}

// The first block is the starting block.
// `locals[i]` has name `LocalName(Name::from_internal(i))`
// `blocks[i]` has name `BbName(Name::from_internal(i))`
//
// if ret == Yes,
//   then _0 is the return local
//   and _1 .. (_n+1) are the locals of the function args.
// if ret == No,
//   then there is no return local
//   and _0 .. _n are the locals of the function args.
pub fn function(ret: Ret, num_args: usize, locals: &[Type], bbs: &[BasicBlock]) -> Function {
    let mut locals: Map<LocalName, Type> = locals
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let name = LocalName(Name::from_internal(i as _));
            (name, *l)
        })
        .collect();

    let args = (0..num_args)
        .map(|x| {
            // `Ret::Yes` shifts the arg locals by one so that they start at one instead of zero.
            let idx = match ret {
                Ret::Yes => x + 1,
                Ret::No => x,
            };

            LocalName(Name::from_internal(idx as _))
        })
        .collect();

    // the ret local has name `0` if it exists.
    let ret = match ret {
        Ret::Yes => {
            assert!(locals.len() > 0);
            let name = LocalName(Name::from_internal(0));
            name
        }
        Ret::No => {
            // Generate a return local of type unit.
            let name = LocalName(Name::from_internal(locals.len().try_to_usize().unwrap() as _));
            locals.try_insert(name, <()>::get_type()).unwrap();
            name
        }
    };

    let blocks = bbs
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let name = BbName(Name::from_internal(i as _));
            (name, *b)
        })
        .collect();

    let start = BbName(Name::from_internal(0));

    Function {
        locals,
        args,
        ret,
        blocks,
        start,
        // For now we use the C ABI for everything since that's what `spawn` needs...
        calling_convention: CallingConvention::C,
    }
}

pub fn block(statements: &[Statement], terminator: Terminator) -> BasicBlock {
    BasicBlock { statements: statements.iter().copied().collect(), terminator }
}

// block!(statement1, statement2, ..., terminator)
// is syntactic sugar for
// block(&[statement1, statement2, ...], terminator)
//
// This macro is evaluated as follows:
// block!(a, b, c)
// block!(@{} a, b, c)
// block!(@{a} b, c)
// block!(@{a, b} c)
// block(&[a, b], c)
//
// This seems necessary, as macros like this
// ($($rest:expr),*, $terminator:expr) => { ... }
// cause `local ambiguity` when called
pub macro block {
    // entry point
    ($($rest:expr),* $(,)?) => {
        block!(@{} $($rest),*)
    },
    (@{$($stmts:expr),*} $terminator:expr) => {
        block(&[$($stmts),*], $terminator)
    },

    // This is just a specialization of the case below.
    // We do not know why it is required separately.
    (@{} $stmt:expr, $($rest:expr),*) => {
        block!(@{$stmt} $($rest),*)
    },
    (@{$($stmts:expr),*} $stmt:expr, $($rest:expr),*) => {
        block!(@{$($stmts),*, $stmt} $($rest),*)
    },
}
