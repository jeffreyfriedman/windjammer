//! Generated-program AST over a UB-free i64 subset: shape, Windjammer printer, reference evaluator.
//!
//! The evaluator is an independent implementation of the intended semantics (wrapping i64,
//! truncating division, value-semantics Copy structs); no Windjammer backend is involved.

use std::collections::BTreeMap;
use std::fmt::Write;

pub const NVARS: usize = 4;
pub const VEC_LEN: i64 = 8;

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    WAdd,
    WSub,
    WMul,
    Xor,
    And,
    Or,
    ModLit(i64),
    DivLit(i64),
    ShrLit(u32),
    ShlLit(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(i64),
    Var(usize),
    Ctr(usize),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Field(usize, bool),
    VecGet(Box<Expr>),
    MapGet(Box<Expr>),
    Call(usize, Box<Expr>, Box<Expr>),
    Tmp(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CmpOp {
    Lt,
    Le,
    Eq,
    Ne,
    Gt,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Assign(usize, Expr),
    AssignField(usize, bool, Expr),
    CopyStruct(usize, usize),
    VecSet(Expr, Expr),
    MapInsert(Expr, Expr),
    If(CmpOp, Expr, Expr, Vec<Stmt>, Vec<Stmt>),
    Repeat(u32, Vec<Stmt>),
    Block(Vec<Stmt>),
    Check(Expr),
    Tmp(usize, Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub init: [i64; NVARS],
    pub stmts: Vec<Stmt>,
}

/// Naming scheme so metamorphic renaming can re-print the same AST with other identifiers.
#[derive(Clone, Debug)]
pub struct Names {
    pub prefix: String,
}

impl Names {
    pub fn plain() -> Names {
        Names { prefix: String::new() }
    }
    pub fn var(&self, i: usize) -> String {
        format!("{}v{i}", self.prefix)
    }
}

pub fn print_expr(e: &Expr, n: &Names) -> String {
    match e {
        Expr::Lit(v) if *v < 0 => format!("(-{}i64)", (*v as i128).abs()),
        Expr::Lit(v) => format!("{v}i64"),
        Expr::Var(i) => n.var(*i),
        Expr::Ctr(d) => format!("{}c{d}", n.prefix),
        Expr::Bin(op, a, b) => print_bin(op, &print_expr(a, n), &print_expr(b, n)),
        Expr::Field(s, a) => format!("{}s{s}.{}", n.prefix, if *a { "a" } else { "b" }),
        Expr::VecGet(i) => format!("{}vec[{}]", n.prefix, idx(&print_expr(i, n))),
        Expr::MapGet(k) => format!("map_get({}m, {})", n.prefix, print_expr(k, n)),
        Expr::Call(f, a, b) => format!("helper{f}({}, {})", print_expr(a, n), print_expr(b, n)),
        Expr::Tmp(t) => format!("{}t{t}", n.prefix),
    }
}

fn idx(e: &str) -> String {
    format!("((({e} % 8i64) + 8i64) % 8i64) as usize")
}

fn print_bin(op: &BinOp, a: &str, b: &str) -> String {
    match op {
        BinOp::WAdd => format!("({a}).wrapping_add({b})"),
        BinOp::WSub => format!("({a}).wrapping_sub({b})"),
        BinOp::WMul => format!("({a}).wrapping_mul({b})"),
        BinOp::Xor => format!("(({a}) ^ ({b}))"),
        BinOp::And => format!("(({a}) & ({b}))"),
        BinOp::Or => format!("(({a}) | ({b}))"),
        BinOp::ModLit(m) => format!("(({a}) % {m}i64)"),
        BinOp::DivLit(d) => format!("(({a}) / {d}i64)"),
        BinOp::ShrLit(k) => format!("(({a}) >> {k})"),
        BinOp::ShlLit(k) => format!("(({a}) << {k})"),
    }
}

fn cmp_str(c: &CmpOp) -> &'static str {
    match c {
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::Gt => ">",
    }
}

pub fn print_stmts(stmts: &[Stmt], depth: usize, n: &Names, out: &mut String) {
    let pad = "    ".repeat(depth + 1);
    for s in stmts {
        print_stmt(s, depth, &pad, n, out);
    }
}

fn print_stmt(s: &Stmt, depth: usize, pad: &str, n: &Names, out: &mut String) {
    let p = &n.prefix;
    match s {
        Stmt::Assign(v, e) => writeln!(out, "{pad}{} = {}", n.var(*v), print_expr(e, n)).unwrap(),
        Stmt::AssignField(si, a, e) => {
            writeln!(out, "{pad}{p}s{si}.{} = {}", if *a { "a" } else { "b" }, print_expr(e, n)).unwrap()
        }
        Stmt::CopyStruct(d, s) => writeln!(out, "{pad}{p}s{d} = {p}s{s}").unwrap(),
        Stmt::VecSet(i, e) => writeln!(out, "{pad}{p}vec[{}] = {}", idx(&print_expr(i, n)), print_expr(e, n)).unwrap(),
        Stmt::MapInsert(k, e) => writeln!(out, "{pad}{p}m.insert({}, {})", print_expr(k, n), print_expr(e, n)).unwrap(),
        Stmt::Tmp(t, e) => writeln!(out, "{pad}let {p}t{t} = {}", print_expr(e, n)).unwrap(),
        Stmt::Check(e) => writeln!(out, "{pad}println(\"{{}}\", {})", print_expr(e, n)).unwrap(),
        Stmt::Block(b) => {
            writeln!(out, "{pad}if true {{").unwrap();
            print_stmts(b, depth + 1, n, out);
            writeln!(out, "{pad}}}").unwrap();
        }
        Stmt::If(c, a, b, t, e) => {
            writeln!(out, "{pad}if {} {} {} {{", print_expr(a, n), cmp_str(c), print_expr(b, n)).unwrap();
            print_stmts(t, depth + 1, n, out);
            writeln!(out, "{pad}}} else {{").unwrap();
            print_stmts(e, depth + 1, n, out);
            writeln!(out, "{pad}}}").unwrap();
        }
        Stmt::Repeat(k, body) => {
            writeln!(out, "{pad}let mut {p}c{depth} = 0i64").unwrap();
            writeln!(out, "{pad}while {p}c{depth} < {k}i64 {{").unwrap();
            print_stmts(body, depth + 1, n, out);
            writeln!(out, "{pad}    {p}c{depth} += 1i64").unwrap();
            writeln!(out, "{pad}}}").unwrap();
        }
    }
}

/// Source text of `fn case_<id>()` (declarations + statements).
pub fn print_case(id: usize, c: &Case, n: &Names) -> String {
    let p = &n.prefix;
    let mut out = format!("fn case_{id}() {{\n");
    for (i, v) in c.init.iter().enumerate() {
        let lit = print_expr(&Expr::Lit(*v), n);
        writeln!(out, "    let mut {} = {lit}", n.var(i)).unwrap();
    }
    writeln!(out, "    let mut {p}vec: Vec<i64> = vec![1i64, 2i64, 3i64, 5i64, 8i64, 13i64, 21i64, 34i64]").unwrap();
    writeln!(out, "    let mut {p}s0 = S {{ a: 1i64, b: 2i64 }}").unwrap();
    writeln!(out, "    let mut {p}s1 = S {{ a: 3i64, b: 4i64 }}").unwrap();
    writeln!(out, "    let mut {p}m: HashMap<i64, i64> = HashMap::new()").unwrap();
    print_stmts(&c.stmts, 0, n, &mut out);
    out.push_str("}\n");
    out
}

pub const PRELUDE: &str = "use std::collections::HashMap\n\nstruct S {\n    a: i64,\n    b: i64,\n}\n\nfn map_get(m: HashMap<i64, i64>, k: i64) -> i64 {\n    match m.get(k) {\n        Some(v) => v,\n        None => 0i64,\n    }\n}\n\nfn helper0(a: i64, b: i64) -> i64 {\n    a.wrapping_mul(31i64).wrapping_add(b)\n}\n\nfn helper1(a: i64, b: i64) -> i64 {\n    (a ^ b).wrapping_sub(7i64)\n}\n\nfn helper2(a: i64, b: i64) -> i64 {\n    a\n}\n\n";

/// A full program running every case; each case's trace is introduced by a `#<id>` marker line.
pub fn print_program(cases: &[(usize, Case)], n: &Names) -> String {
    let mut out = String::from(PRELUDE);
    for (id, c) in cases {
        out.push_str(&print_case(*id, c, n));
        out.push('\n');
    }
    out.push_str("fn main() {\n");
    for (id, _) in cases {
        writeln!(out, "    println(\"#{id}\")\n    case_{id}()").unwrap();
    }
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------------------------
// Reference evaluator
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct State {
    vars: [i64; NVARS],
    ctr: Vec<i64>,
    vec: Vec<i64>,
    s: [(i64, i64); 2],
    m: BTreeMap<i64, i64>,
    tmp: BTreeMap<usize, i64>,
}

fn eval_bin(op: &BinOp, a: i64, b: i64) -> i64 {
    match op {
        BinOp::WAdd => a.wrapping_add(b),
        BinOp::WSub => a.wrapping_sub(b),
        BinOp::WMul => a.wrapping_mul(b),
        BinOp::Xor => a ^ b,
        BinOp::And => a & b,
        BinOp::Or => a | b,
        BinOp::ModLit(m) => a % m,
        BinOp::DivLit(d) => a / d,
        BinOp::ShrLit(k) => a >> k,
        BinOp::ShlLit(k) => a << k,
    }
}

fn helper(f: usize, a: i64, b: i64) -> i64 {
    match f {
        0 => a.wrapping_mul(31).wrapping_add(b),
        1 => (a ^ b).wrapping_sub(7),
        _ => a,
    }
}

fn eval_expr(e: &Expr, st: &State) -> i64 {
    match e {
        Expr::Lit(v) => *v,
        Expr::Var(i) => st.vars[*i],
        Expr::Ctr(d) => st.ctr.get(*d).copied().unwrap_or(0),
        Expr::Bin(op, a, b) => eval_bin(op, eval_expr(a, st), eval_expr(b, st)),
        Expr::Field(s, a) => if *a { st.s[*s].0 } else { st.s[*s].1 },
        Expr::VecGet(i) => st.vec[(((eval_expr(i, st) % 8) + 8) % 8) as usize],
        Expr::MapGet(k) => st.m.get(&eval_expr(k, st)).copied().unwrap_or(0),
        Expr::Call(f, a, b) => helper(*f, eval_expr(a, st), eval_expr(b, st)),
        Expr::Tmp(t) => st.tmp[t],
    }
}

fn eval_stmts(stmts: &[Stmt], depth: usize, st: &mut State, trace: &mut Vec<String>) {
    for s in stmts {
        match s {
            Stmt::Assign(v, e) => st.vars[*v] = eval_expr(e, st),
            Stmt::AssignField(si, a, e) => {
                let v = eval_expr(e, st);
                if *a { st.s[*si].0 = v } else { st.s[*si].1 = v }
            }
            Stmt::CopyStruct(d, src) => st.s[*d] = st.s[*src],
            Stmt::VecSet(i, e) => {
                let (ix, v) = ((((eval_expr(i, st) % 8) + 8) % 8) as usize, eval_expr(e, st));
                st.vec[ix] = v;
            }
            Stmt::MapInsert(k, e) => {
                let (k, v) = (eval_expr(k, st), eval_expr(e, st));
                st.m.insert(k, v);
            }
            Stmt::Tmp(t, e) => {
                let v = eval_expr(e, st);
                st.tmp.insert(*t, v);
            }
            Stmt::Check(e) => trace.push(eval_expr(e, st).to_string()),
            Stmt::Block(b) => eval_stmts(b, depth + 1, st, trace),
            Stmt::If(c, a, b, t, e) => {
                let (x, y) = (eval_expr(a, st), eval_expr(b, st));
                let hit = match c {
                    CmpOp::Lt => x < y,
                    CmpOp::Le => x <= y,
                    CmpOp::Eq => x == y,
                    CmpOp::Ne => x != y,
                    CmpOp::Gt => x > y,
                };
                eval_stmts(if hit { t } else { e }, depth + 1, st, trace);
            }
            Stmt::Repeat(k, body) => {
                while st.ctr.len() <= depth {
                    st.ctr.push(0);
                }
                for i in 0..*k as i64 {
                    st.ctr[depth] = i;
                    eval_stmts(body, depth + 1, st, trace);
                }
            }
        }
    }
}

/// The expected trace lines of one case.
pub fn eval_case(c: &Case) -> Vec<String> {
    let mut st = State {
        vars: c.init,
        ctr: Vec::new(),
        vec: vec![1, 2, 3, 5, 8, 13, 21, 34],
        s: [(1, 2), (3, 4)],
        m: BTreeMap::new(),
        tmp: BTreeMap::new(),
    };
    let mut trace = Vec::new();
    eval_stmts(&c.stmts, 0, &mut st, &mut trace);
    trace
}

/// Expected stdout lines of a whole program (`#id` markers + traces).
pub fn eval_program(cases: &[(usize, Case)]) -> Vec<String> {
    let mut out = Vec::new();
    for (id, c) in cases {
        out.push(format!("#{id}"));
        out.extend(eval_case(c));
    }
    out
}
