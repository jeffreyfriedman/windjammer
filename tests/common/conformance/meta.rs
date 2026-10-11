//! Semantic-preserving transformations. Each must leave every backend's trace unchanged.

use super::gen_ast::*;

/// Prefix every local identifier (rename locals).
pub fn rename_names() -> Names {
    Names { prefix: "zz_".to_string() }
}

fn reads_writes(s: &Stmt) -> (Vec<usize>, Vec<usize>, bool) {
    // (vars read, vars written, touches shared state: vec/map/struct/check/control flow)
    fn vars_of(e: &Expr, out: &mut Vec<usize>, shared: &mut bool) {
        match e {
            Expr::Var(i) => out.push(*i),
            Expr::Bin(_, a, b) | Expr::Call(_, a, b) => {
                vars_of(a, out, shared);
                vars_of(b, out, shared);
            }
            Expr::VecGet(a) | Expr::MapGet(a) => {
                *shared = true;
                vars_of(a, out, shared);
            }
            Expr::Field(..) | Expr::Ctr(_) => *shared = true,
            Expr::Lit(_) | Expr::Tmp(_) => {}
        }
    }
    let (mut r, mut shared) = (Vec::new(), false);
    match s {
        Stmt::Assign(v, e) => {
            vars_of(e, &mut r, &mut shared);
            (r, vec![*v], shared)
        }
        _ => (r, vec![], true),
    }
}

fn independent(a: &Stmt, b: &Stmt) -> bool {
    let (ra, wa, sa) = reads_writes(a);
    let (rb, wb, sb) = reads_writes(b);
    !sa && !sb && !wa.iter().any(|w| rb.contains(w) || wb.contains(w)) && !wb.iter().any(|w| ra.contains(w))
}

/// Swap adjacent independent scalar assignments (top level and nested).
pub fn reorder_independent(stmts: &[Stmt]) -> Vec<Stmt> {
    let mut v: Vec<Stmt> = stmts.iter().map(reorder_nested).collect();
    let mut i = 0;
    while i + 1 < v.len() {
        if independent(&v[i], &v[i + 1]) {
            v.swap(i, i + 1);
            i += 2;
        } else {
            i += 1;
        }
    }
    v
}

fn reorder_nested(s: &Stmt) -> Stmt {
    match s {
        Stmt::If(c, a, b, t, e) => Stmt::If(c.clone(), a.clone(), b.clone(), reorder_independent(t), reorder_independent(e)),
        Stmt::Repeat(k, b) => Stmt::Repeat(*k, reorder_independent(b)),
        Stmt::Block(b) => Stmt::Block(reorder_independent(b)),
        other => other.clone(),
    }
}

/// Wrap every top-level statement in its own block (`if true { ... }`).
pub fn wrap_in_blocks(stmts: &[Stmt]) -> Vec<Stmt> {
    stmts.iter().map(|s| Stmt::Block(vec![s.clone()])).collect()
}

/// Split `v = a op b` into `let t = a; v = t op b` (fresh temporary ids start at `first_tmp`).
pub fn split_into_temporaries(stmts: &[Stmt], next_tmp: &mut usize) -> Vec<Stmt> {
    stmts
        .iter()
        .flat_map(|s| match s {
            Stmt::Assign(v, Expr::Bin(op, a, b)) => {
                let t = *next_tmp;
                *next_tmp += 1;
                vec![Stmt::Tmp(t, (**a).clone()), Stmt::Assign(*v, Expr::Bin(op.clone(), Box::new(Expr::Tmp(t)), b.clone()))]
            }
            other => vec![other.clone()],
        })
        .collect()
}

/// Extract an assigned expression through an identity helper call (`helper2(e, 0) == e`).
pub fn extract_identity(stmts: &[Stmt]) -> Vec<Stmt> {
    stmts
        .iter()
        .map(|s| match s {
            Stmt::Assign(v, e) => Stmt::Assign(*v, Expr::Call(2, Box::new(e.clone()), Box::new(Expr::Lit(0)))),
            other => other.clone(),
        })
        .collect()
}
