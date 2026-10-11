//! Delta-debugging shrinker over statement trees.
//!
//! All variables are predeclared, so removing any statement keeps the program well-formed.

use super::gen_ast::{Case, Stmt};

fn children(s: &Stmt) -> Vec<Vec<Stmt>> {
    match s {
        Stmt::If(_, _, _, t, e) => vec![t.clone(), e.clone()],
        Stmt::Repeat(_, b) | Stmt::Block(b) => vec![b.clone()],
        _ => vec![],
    }
}

fn rebuild(s: &Stmt, which: usize, new: Vec<Stmt>) -> Stmt {
    match s {
        Stmt::If(c, a, b, t, e) => {
            if which == 0 { Stmt::If(c.clone(), a.clone(), b.clone(), new, e.clone()) } else { Stmt::If(c.clone(), a.clone(), b.clone(), t.clone(), new) }
        }
        Stmt::Repeat(k, _) => Stmt::Repeat(*k, new),
        Stmt::Block(_) => Stmt::Block(new),
        other => other.clone(),
    }
}

/// All single-step reductions of a statement list: drop one statement, hoist a body, or recurse.
fn reductions(stmts: &[Stmt]) -> Vec<Vec<Stmt>> {
    let mut out = Vec::new();
    for i in 0..stmts.len() {
        let mut dropped = stmts.to_vec();
        dropped.remove(i);
        out.push(dropped);
        for body in children(&stmts[i]) {
            let mut hoisted = stmts.to_vec();
            hoisted.splice(i..=i, body);
            out.push(hoisted);
        }
        for (w, body) in children(&stmts[i]).into_iter().enumerate() {
            for red in reductions(&body) {
                let mut v = stmts.to_vec();
                v[i] = rebuild(&stmts[i], w, red);
                out.push(v);
            }
        }
    }
    out
}

fn size(stmts: &[Stmt]) -> usize {
    stmts.iter().map(|s| 1 + children(s).iter().map(|c| size(c)).sum::<usize>()).sum()
}

/// Greedy fixpoint: keep any reduction for which `still_fails` holds.
pub fn shrink(case: &Case, mut still_fails: impl FnMut(&Case) -> bool) -> Case {
    let mut cur = case.clone();
    loop {
        let mut progressed = false;
        for red in reductions(&cur.stmts) {
            if size(&red) >= size(&cur.stmts) {
                continue;
            }
            let cand = Case { init: cur.init, stmts: red };
            if still_fails(&cand) {
                cur = cand;
                progressed = true;
                break;
            }
        }
        if !progressed {
            return cur;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::gen_ast::{Expr, Stmt};

    #[test]
    fn shrinks_to_the_one_culprit_statement() {
        let culprit = Stmt::Check(Expr::Lit(777));
        let mut stmts: Vec<Stmt> = (0..6).map(|i| Stmt::Check(Expr::Lit(i))).collect();
        stmts.push(Stmt::Block(vec![Stmt::Repeat(2, vec![culprit.clone(), Stmt::Check(Expr::Lit(5))])]));
        let case = Case { init: [0; 4], stmts };
        let has = |c: &Case| format!("{:?}", c.stmts).contains("777");
        let small = shrink(&case, has);
        assert_eq!(small.stmts, vec![culprit]);
    }
}
