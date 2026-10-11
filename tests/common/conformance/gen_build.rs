//! Seeded grammar-based generation of well-typed, UB-free cases.

use super::gen_ast::*;
use super::rng::Rng;

const INTERESTING: [i64; 8] = [0, 1, -1, 2, 255, 9007199254740993, -9007199254740993, i64::MAX];

fn gen_lit(r: &mut Rng) -> i64 {
    if r.chance(1, 3) {
        *r.pick(&INTERESTING)
    } else {
        r.below(2001) as i64 - 1000
    }
}

fn gen_bin_op(r: &mut Rng) -> BinOp {
    match r.below(10) {
        0 => BinOp::WAdd,
        1 => BinOp::WSub,
        2 => BinOp::WMul,
        3 => BinOp::Xor,
        4 => BinOp::And,
        5 => BinOp::Or,
        6 => BinOp::ModLit(1 + r.below(1000) as i64),
        7 => BinOp::DivLit(2 + r.below(100) as i64),
        8 => BinOp::ShrLit(r.below(63) as u32),
        _ => BinOp::ShlLit(r.below(63) as u32),
    }
}

pub fn gen_expr(r: &mut Rng, depth: u32, loops: usize) -> Expr {
    if depth == 0 || r.chance(1, 4) {
        return match r.below(5) {
            0 => Expr::Lit(gen_lit(r)),
            1 if loops > 0 => Expr::Ctr(r.below(loops as u64) as usize),
            2 => Expr::Field(r.below(2) as usize, r.chance(1, 2)),
            _ => Expr::Var(r.below(NVARS as u64) as usize),
        };
    }
    let sub = |r: &mut Rng| Box::new(gen_expr(r, depth - 1, loops));
    match r.below(8) {
        0 => Expr::VecGet(sub(r)),
        1 => Expr::MapGet(sub(r)),
        2 => Expr::Call(r.below(2) as usize, sub(r), sub(r)),
        _ => Expr::Bin(gen_bin_op(r), sub(r), sub(r)),
    }
}

fn gen_cmp(r: &mut Rng) -> CmpOp {
    match r.below(5) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Eq,
        3 => CmpOp::Ne,
        _ => CmpOp::Gt,
    }
}

pub fn gen_stmt(r: &mut Rng, depth: u32, loops: usize) -> Stmt {
    let e = |r: &mut Rng| gen_expr(r, 3, loops);
    let nested = depth > 0;
    match r.below(if nested { 12 } else { 8 }) {
        0 | 1 => Stmt::Assign(r.below(NVARS as u64) as usize, e(r)),
        2 => Stmt::AssignField(r.below(2) as usize, r.chance(1, 2), e(r)),
        3 => {
            let d = r.below(2) as usize;
            Stmt::CopyStruct(d, 1 - d)
        }
        4 => Stmt::VecSet(e(r), e(r)),
        5 => Stmt::MapInsert(e(r), e(r)),
        6 | 7 => Stmt::Check(e(r)),
        8 => Stmt::If(gen_cmp(r), e(r), e(r), gen_block(r, depth - 1, loops, 3), gen_block(r, depth - 1, loops, 2)),
        9 | 10 => Stmt::Repeat(1 + r.below(5) as u32, gen_block(r, depth - 1, loops + 1, 4)),
        _ => Stmt::Block(gen_block(r, depth - 1, loops, 3)),
    }
}

pub fn gen_block(r: &mut Rng, depth: u32, loops: usize, max: u64) -> Vec<Stmt> {
    let n = 1 + r.below(max);
    (0..n).map(|_| gen_stmt(r, depth, loops)).collect()
}

/// One case from a seed. Always ends with checks of all scalar state so divergence is observable.
pub fn gen_case(seed: u64) -> Case {
    let mut r = Rng::new(seed);
    let mut init = [0i64; NVARS];
    for v in init.iter_mut() {
        *v = gen_lit(&mut r);
    }
    let mut stmts = gen_block(&mut r, 2, 0, 10);
    for i in 0..NVARS {
        stmts.push(Stmt::Check(Expr::Var(i)));
    }
    stmts.push(Stmt::Check(Expr::Field(0, true)));
    stmts.push(Stmt::Check(Expr::Field(1, false)));
    stmts.push(Stmt::Check(Expr::VecGet(Box::new(Expr::Lit(3)))));
    Case { init, stmts }
}
