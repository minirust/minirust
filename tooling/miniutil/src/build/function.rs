use crate::build::*;

pub fn fn_ptr(x: u32) -> ValueExpr {
    let x = Name::new(x as _);
    let x = FnName(x);
    let x = Constant::FnPointer(x);
    let x = ValueExpr::Constant(x, Type::Ptr(PtrType::FnPtr));
    x
}

// fns[0] is the start function.
// fns[i] has name FnName(Name::new(i))
pub fn program(fns: &[Function]) -> Program {
    let mut functions = Map::new();
    for (i, f) in fns.iter().enumerate() {
        functions.insert(FnName(Name::new(i as _)), *f);
    }
    Program {
        functions,
        start: FnName(Name::new(0)),
        globals: Default::default(),
    }
}

// whether a function returns or not.
pub enum Ret { Yes, No }

// if ret == Yes, then _0 is the return local.
// the first block is the starting block.
// locals[i] has name LocalName(Name::new(i))
// blocks[i] has name BbName(Name::new(i))
pub fn function(ret: Ret, num_args: usize, locs: &[PlaceType], bbs: &[BasicBlock]) -> Function {
    let mut locals = Map::new();
    for (i, l) in locs.iter().enumerate() {
        locals.insert(LocalName(Name::new(i as _)), *l);
    }

    let args = (0..num_args).map(|x| {
        let idx = match ret {
            Ret::Yes => x+1,
            Ret::No => x,
        };

        (LocalName(Name::new(idx as _)), ArgAbi::Register)
    }).collect();

    let ret = match ret {
        Ret::Yes => {
            assert!(locs.len() > 0);
            Some((LocalName(Name::new(0)), ArgAbi::Register))
        }
        Ret::No => None,
    };

    let mut blocks = Map::new();
    for (i, b) in bbs.iter().enumerate() {
        blocks.insert(BbName(Name::new(i as _)), *b);
    }

    let start = BbName(Name::new(0));

    Function {
        locals,
        args,
        ret,
        blocks,
        start,
    }
}

pub fn block(statements: &[Statement], terminator: Terminator) -> BasicBlock {
    BasicBlock {
        statements: statements.iter().copied().collect(),
        terminator,
    }
}

// like `block`, but a sometimes nicer API.
pub fn block2(l: &[&dyn ToStmtTerm]) -> BasicBlock {
    let StmtTerm::Term(term) = l.last().unwrap().to_stmt_term() else { panic!() };
    let mut stmts = Vec::new();
    for x in &l[0..l.len() - 1] {
        let StmtTerm::Stmt(stmt) = x.to_stmt_term() else { panic!() };
        stmts.push(stmt);
    }

    BasicBlock {
        statements: stmts.iter().copied().collect(),
        terminator: term,
    }
}

pub enum StmtTerm {
    Stmt(Statement),
    Term(Terminator),
}

pub trait ToStmtTerm {
    fn to_stmt_term(&self) -> StmtTerm;
}

impl ToStmtTerm for Statement {
    fn to_stmt_term(&self) -> StmtTerm {
        StmtTerm::Stmt(*self)
    }
}

impl ToStmtTerm for Terminator {
    fn to_stmt_term(&self) -> StmtTerm {
        StmtTerm::Term(*self)
    }
}
