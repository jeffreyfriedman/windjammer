//! Seeded generator of well-typed, UB-free Rust programs exercising every
//! optimizer pass, each paired with an expected value computed natively here
//! (independent of the generated code). Includes "trap" templates that a
//! careless rewrite would break.

#![allow(dead_code)]

use super::support;

pub struct Xs(pub u64);

impl Xs {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Clone)]
pub struct Case {
    pub items: String,
    pub body: String,
    pub expected: u64,
}

const TEMPLATES: u64 = 10;

fn fold_str(s: &str) -> u64 {
    s.bytes().fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64))
}

pub fn gen_case(seed: u64, id: usize) -> Case {
    let mut r = Xs(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    r.next();
    match r.below(TEMPLATES) {
        0 => fill(&mut r, id),
        1 => fill_expr_bound(&mut r, id),
        2 => break_loop(&mut r, id),
        3 => start_bound(&mut r, id),
        4 => append_plain(&mut r, id),
        5 => append_self_reference(&mut r, id),
        6 => clone_traps(&mut r, id),
        7 => cast_chains(&mut r, id),
        8 => shadow_traps(&mut r, id),
        _ => struct_helpers(&mut r, id),
    }
}

fn wrap(id: usize, body: &str, expected: u64) -> Case {
    Case { items: String::new(), body: format!("fn case_{id}() -> u64 {{\n{body}\n}}\n"), expected }
}

fn fill(r: &mut Xs, id: usize) -> Case {
    let (n, m, a) = (r.below(300), r.below(1000) + 1, r.below(50));
    let mut exp = 7u64;
    for i in 0..n {
        exp = exp.wrapping_mul(31).wrapping_add(i.wrapping_mul(m) + a);
    }
    let body = format!(
        "    let n: usize = {n}_usize;\n    let n_copy = n.clone();\n    let mut v: Vec<u64> = Vec::new();\n    for i in 0_usize..n {{\n        v.push((i as u64).wrapping_mul({m}_u64) + {a}_u64);\n    }}\n    let mut s: u64 = 7_u64 + n_copy as u64 * 0_u64;\n    for x in v.iter() {{\n        s = s.wrapping_mul(31_u64).wrapping_add(*x);\n    }}\n    s"
    );
    wrap(id, &body, exp)
}

fn fill_expr_bound(r: &mut Xs, id: usize) -> Case {
    let (n, k) = (r.below(20), r.below(9));
    let mut exp = 0u64;
    let mut cnt = 0u64;
    for i in 0..(n * n + k) {
        exp = exp.wrapping_add(i * 3);
        cnt += 1;
    }
    let body = format!(
        "    let n: usize = {n}_usize;\n    let mut v: Vec<u64> = Vec::new();\n    for i in 0_usize..n * n + {k}_usize {{\n        v.push(i as u64 * 3_u64);\n    }}\n    let mut s = 0_u64;\n    for x in v.iter() {{\n        s = s.wrapping_add(*x);\n    }}\n    s + v.len() as u64 * 0_u64 + {cnt}_u64 * 0_u64"
    );
    wrap(id, &body, exp)
}

fn break_loop(r: &mut Xs, id: usize) -> Case {
    let (b, m) = (r.below(50), r.below(90) + 1);
    let exp = (0..=b).fold(0u64, |s, i| s.wrapping_add(i * m));
    let body = format!(
        "    let max: usize = usize::MAX / 4_usize;\n    let mut v: Vec<u64> = Vec::new();\n    for i in 0_usize..max {{\n        v.push(i as u64 * {m}_u64);\n        if i == {b}_usize {{\n            break;\n        }}\n    }}\n    let mut s = 0_u64;\n    for x in v.iter() {{\n        s = s.wrapping_add(*x);\n    }}\n    s"
    );
    wrap(id, &body, exp)
}

fn start_bound(r: &mut Xs, id: usize) -> Case {
    let (lo, hi) = (r.below(20), r.below(20));
    let exp = (lo..hi).fold(0u64, |s, i| s.wrapping_add(i * i));
    let body = format!(
        "    let lo: usize = {lo}_usize;\n    let hi: usize = {hi}_usize;\n    let mut v: Vec<u64> = Vec::new();\n    for i in lo..hi {{\n        v.push((i * i) as u64);\n    }}\n    let mut s = 0_u64;\n    for x in v.iter() {{\n        s = s.wrapping_add(*x);\n    }}\n    s"
    );
    wrap(id, &body, exp)
}

fn append_plain(r: &mut Xs, id: usize) -> Case {
    let (k, m) = (r.below(60), r.below(500) + 1);
    let mut s = String::new();
    for i in 0..k {
        s = format!("{}{}{}", s, i * m, ",");
    }
    let body = format!(
        "    let mut out = String::new();\n    for i in 0_u64..{k}_u64 {{\n        out = format!(\"{{}}{{}}{{}}\", out, i * {m}_u64, \",\");\n    }}\n    let mut h = 7_u64;\n    for b in out.bytes() {{\n        h = h.wrapping_mul(31_u64).wrapping_add(b as u64);\n    }}\n    h"
    );
    wrap(id, &body, fold_str(&s))
}

fn append_self_reference(r: &mut Xs, id: usize) -> Case {
    let k = r.below(6) + 1;
    let mut s = String::from("x");
    for _ in 0..k {
        s = format!("{}{}", s, s);
    }
    let body = format!(
        "    let mut u = String::from(\"x\");\n    for _i in 0_u64..{k}_u64 {{\n        u = format!(\"{{}}{{}}\", u, u);\n    }}\n    let mut h = 7_u64;\n    for b in u.bytes() {{\n        h = h.wrapping_mul(31_u64).wrapping_add(b as u64);\n    }}\n    h"
    );
    wrap(id, &body, fold_str(&s))
}

fn clone_traps(r: &mut Xs, id: usize) -> Case {
    let (a, b, c) = (r.below(100), r.below(100), r.below(100));
    // snapshot keeps the pre-push contents; temp clone is a no-op.
    let exp = (a + b) * 10 + (a + b + c) + 6 + 2 * 100 + 1000;
    let body = format!(
        "    let mut v: Vec<u64> = vec![{a}_u64, {b}_u64];\n    let snap = v.clone();\n    v.push({c}_u64);\n    let t = vec![1_u64, 2_u64, 3_u64].clone();\n    let s1: u64 = snap.iter().sum();\n    let s2: u64 = v.iter().sum();\n    let s3: u64 = t.iter().sum();\n    let wrapped = vec![snap.clone()];\n    s1 * 10_u64 + s2 + s3 + snap.len() as u64 * 100_u64 + wrapped.len() as u64 * 1000_u64"
    );
    wrap(id, &body, exp)
}

fn cast_chains(r: &mut Xs, id: usize) -> Case {
    let k = r.below(70000) as i64 - 35000;
    let n = r.below(1000) as usize;
    let exp = ((k as i32) as i32) as i64 as u64 + (k as u8) as u32 as u64 + (n as usize) as u64;
    let body = format!(
        "    let k: i64 = {k}_i64;\n    let n: usize = {n}_usize;\n    let a = (k as i32) as i32;\n    let b = (k as u8) as u32;\n    let m = n as usize;\n    (a as i64) as u64 + b as u64 + m as u64"
    );
    wrap(id, &body, exp)
}

fn shadow_traps(r: &mut Xs, id: usize) -> Case {
    let (n, p) = (r.below(100000) as usize, r.below(9));
    let len = n.to_string().len() as u64;
    let exp = len * 3 + p;
    let body = format!(
        "    let n: usize = {n}_usize;\n    let s = n.to_string();\n    let n = s;\n    let t = n.clone();\n    let label = format!(\"{{}}\", t.clone());\n    let mut total = n.len() as u64 + t.len() as u64 + label.len() as u64;\n    {{\n        let n = {p}_u64;\n        let m = n.clone();\n        total += m;\n    }}\n    total"
    );
    wrap(id, &body, exp)
}

fn struct_helpers(r: &mut Xs, id: usize) -> Case {
    let (a, b) = (r.below(200), r.below(100000));
    // `Used` keeps its to_bytes (referenced); `Spare` loses both helpers.
    // repr(C) layout of Lay is observed through size_of -> must stay 12.
    let items = format!(
        "#[derive(Clone, Copy)]\n#[repr(C)]\nstruct Used{id} {{\n    a: u8,\n    b: u32,\n}}\nimpl Used{id} {{\n    pub fn to_bytes(&self) -> Vec<u8> {{\n        let mut v = Vec::new();\n        v.push(self.a);\n        v.extend_from_slice(&self.b.to_le_bytes());\n        v\n    }}\n}}\n#[derive(Clone, Copy)]\n#[repr(C)]\nstruct Lay{id} {{\n    a: u8,\n    b: u32,\n    c: u8,\n}}\n"
    );
    let bytes_sum = a + (0..4).map(|i| (b >> (8 * i)) & 255).sum::<u64>();
    let exp = bytes_sum + 12;
    let body = format!(
        "    let u = Used{id} {{ a: {a}_u8, b: {b}_u32 }};\n    let mut s = 0_u64;\n    for x in u.to_bytes() {{\n        s += x as u64;\n    }}\n    let l = Lay{id} {{ a: 1_u8, b: 2_u32, c: 3_u8 }};\n    s + std::mem::size_of::<Lay{id}>() as u64 + (l.a as u64 + l.b as u64 + l.c as u64) * 0_u64"
    );
    let mut c = wrap(id, &body, exp);
    c.items = items;
    c
}

pub fn program(cases: &[Case]) -> String {
    let mut s = String::new();
    for c in cases {
        s.push_str(&c.items);
        s.push_str(&c.body);
    }
    s.push_str("fn main() {\n");
    for c in cases {
        let id = c.body.split("fn case_").nth(1).unwrap().split('(').next().unwrap();
        s.push_str(&format!("    println!(\"{{}}\", case_{id}());\n"));
    }
    s.push_str("}\n");
    s
}

pub fn expected_lines(cases: &[Case]) -> Vec<String> {
    cases.iter().map(|c| c.expected.to_string()).collect()
}

/// Run the generated program once (no seed argument) and split stdout.
pub fn run_program(src: &str, dir: &std::path::Path, name: &str) -> Result<Vec<String>, String> {
    let bin = support::compile(src, dir, name)?;
    let out = std::process::Command::new(&bin).output().unwrap();
    if !out.status.success() {
        return Err(format!("CRASH: {}", String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect())
}

/// Does optimizing `cases` with `cfg` yield a program that fails (compile
/// error, crash, or output differing from the native expectation)?
pub fn fails(cases: &[Case], cfg: &windjammer::optimizer::OptimizationConfig, dir: &std::path::Path, name: &str) -> Option<String> {
    let src = windjammer::optimizer::run(cfg, &program(cases)).code;
    match run_program(&src, dir, name) {
        Err(e) => Some(e),
        Ok(got) if got != expected_lines(cases) => Some("output differs from native expectation".into()),
        Ok(_) => None,
    }
}

/// Delta-debugging: smallest subset of `cases` that still fails.
pub fn shrink(cases: &[Case], cfg: &windjammer::optimizer::OptimizationConfig, dir: &std::path::Path) -> Vec<Case> {
    let mut cur: Vec<Case> = cases.to_vec();
    let mut chunk = cur.len() / 2;
    while chunk >= 1 {
        let mut i = 0;
        while i < cur.len() {
            let mut cand = cur.clone();
            cand.drain(i..(i + chunk).min(cur.len()));
            if !cand.is_empty() && fails(&cand, cfg, dir, "shrink").is_some() {
                cur = cand;
            } else {
                i += chunk;
            }
        }
        chunk /= 2;
    }
    cur
}
