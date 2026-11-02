//! Packed vectors (`interpreter/vecbody.rs`) answer every builtin exactly as unpacked ones: generated programs
//! build the same vector twice, once by `push` (stored packed: one scalar type) and once as a list literal
//! (stored as values), mutate both through a random sequence of the builtins that write (by reference or in
//! place) and print after every step. The packed program repacks before each step (most writes unpack for good,
//! so without it only the first step would see a packed body) and reads each result element by element too. Both
//! must print the same, under the walker and the resolved engine (with every loop entering machine code at its
//! first iteration), so a write path that forgot to unpack shows up as a difference.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// One element type: its declared type, a literal generator and the operations that make sense for it.
struct Elem {
    ty: &'static str,
    lit: fn(&mut Rng) -> String,
    ops: &'static [&'static str],
}

const COMMON: &[&str] = &[
    "v.reverse()",
    "if v.len() > 3 { v.swap(1, 3) }",
    "v.truncate(7)",
    "v.insert(0, X)",
    "if v.len() > 0 { v.remove(0) }",
    "if v.len() > 1 { v.swap_remove(1) }",
    "v.dedup()",
    "if v.len() > 2 { v.rotate_left(2) }",
    "if v.len() > 2 { v.rotate_right(1) }",
    "v.push(X)",
    "if v.len() > 0 { v.pop() }",
    "if v.len() > 0 { v[0] = X }",
    "v.resize(9, X)",
    "v.extend([X, X])",
    "if v.len() > 4 { let tail = v.split_off(3)\n    println(\"${tail:?}\") }",
    "let mut w: Vec<TY> = [X]\n    v.append(w)",
    "if v.len() > 2 { let gone: Vec<TY> = v.drain(1..2).collect()\n    println(\"${gone:?}\") }",
    "v.fill(X)",
    "v.clear()",
];

fn int_lit(r: &mut Rng) -> String {
    (r.below(40) as i64 - 10).to_string()
}
fn float_lit(r: &mut Rng) -> String {
    ["0.5", "-1.25", "3.0", "0.0", "2.5"][r.below(5) as usize].to_string()
}
fn bool_lit(r: &mut Rng) -> String {
    ["true", "false"][r.below(2) as usize].to_string()
}

const ELEMS: &[Elem] = &[
    Elem { ty: "int", lit: int_lit, ops: &["v.sort()", "v.retain(|x| *x % 3 != 0)", "v.sort_by(|a, b| b.cmp(a))", "for x in &mut v {\n        *x += 1\n    }", "if v.len() > 0 { v[0] += 5 }"] },
    Elem { ty: "i32", lit: int_lit, ops: &["v.sort()", "v.retain(|x| *x > 0)", "for x in &mut v {\n        *x *= 2\n    }"] },
    Elem { ty: "f64", lit: float_lit, ops: &["v.retain(|x| *x > 0.0)", "for x in &mut v {\n        *x = *x * 2.0\n    }"] },
    Elem { ty: "bool", lit: bool_lit, ops: &["v.retain(|x| *x)", "for x in &mut v {\n        *x = !*x\n    }"] },
];

fn program(seed: u64, packed: bool) -> String {
    let mut r = Rng(seed.wrapping_mul(0x9e3779b97f4a7c15) | 1);
    let e = &ELEMS[r.below(ELEMS.len() as u64) as usize];
    let items: Vec<String> = (0..3 + r.below(6)).map(|_| (e.lit)(&mut r)).collect();
    let build = if packed {
        format!("let mut v: Vec<{t}> = Vec::new()\n    for x in [{xs}] {{\n        v.push(x)\n    }}", t = e.ty, xs = items.join(", "))
    } else {
        format!("let mut v: Vec<{t}> = [{xs}]", t = e.ty, xs = items.join(", "))
    };
    let mut body = String::new();
    for _ in 0..6 + r.below(8) {
        let all = COMMON.len() + e.ops.len();
        let k = r.below(all as u64) as usize;
        let op = if k < COMMON.len() { COMMON[k] } else { e.ops[k - COMMON.len()] };
        let mut op = op.replace("TY", e.ty);
        while op.contains('X') {
            op = op.replacen('X', &(e.lit)(&mut r), 1);
        }
        // every step starts from a freshly packed vector whose read cache is filled (`{v:?}`), then reads
        // the result both through the cache and element by element (the packed form)
        let repack = if packed { "    v = repack(v)\n" } else { "" };
        body += &format!("{repack}    println(\"${{v:?}}\")\n    {op}\n    println(\"${{v:?}} ${{v.len()}}\")\n    if v.len() > 0 {{ println(\"${{v[0]}} ${{v[v.len() - 1]}}\") }}\n");
    }
    // a packed vector read element by element in a hot loop, then written by reference
    body += "    let mut i = 0\n    let mut n = 0\n    while i < v.len() * 50 {\n        if v.len() > 0 && v[i % v.len()] == v[0] { n += 1 }\n        i += 1\n    }\n    println(\"${n}\")\n";
    let repack = format!("fn repack(v: Vec<{t}>) -> Vec<{t}> {{\n    let mut w: Vec<{t}> = Vec::new()\n    for x in v {{\n        w.push(x)\n    }}\n    w\n}}\n\n", t = e.ty);
    format!("{repack}fn main() {{\n    {build}\n    println(\"${{v:?}}\")\n{body}}}\n")
}

fn run(path: &Path, engine: &str, eager: bool) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut c = Command::new(WJ);
    c.args(["run", "--interpret", "--no-lint", "--engine", engine]).arg(path).stdin(std::process::Stdio::null());
    c.env_remove("WJ_JIT_LOOP_HOT");
    if eager {
        c.env("WJ_JIT_LOOP_HOT", "0");
    }
    let o = c.output().unwrap();
    (o.stdout, o.stderr, o.status.code())
}

#[test]
fn packed_vectors_answer_every_writing_builtin_like_unpacked_ones() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut diffs = Vec::new();
    let mut clean = 0;
    for seed in 1..=80u64 {
        let (pp, up) = (tmp.path().join(format!("p{seed}.wj")), tmp.path().join(format!("u{seed}.wj")));
        std::fs::write(&pp, program(seed, true)).unwrap();
        std::fs::write(&up, program(seed, false)).unwrap();
        let oracle = run(&up, "walker", false);
        clean += (oracle.2 == Some(0)) as usize;
        for (label, p, engine, eager) in [("packed walker", &pp, "walker", false), ("packed resolved", &pp, "resolved", false), ("packed eager jit", &pp, "resolved", true), ("unpacked resolved", &up, "resolved", true)] {
            let o = run(p, engine, eager);
            // the source differs only in how the vector is built, so the outputs must be identical
            if o.0 != oracle.0 || o.2 != oracle.2 {
                diffs.push(format!("seed {seed} ({label}):\n{}\n--- unpacked walker {:?}\n{}{}\n--- {label} {:?}\n{}{}", program(seed, true), oracle.2, String::from_utf8_lossy(&oracle.0), String::from_utf8_lossy(&oracle.1), o.2, String::from_utf8_lossy(&o.0), String::from_utf8_lossy(&o.1)));
            }
        }
    }
    assert!(clean >= 60, "only {clean} of 80 generated programs ran cleanly");
    assert!(diffs.is_empty(), "{} programs differ:\n{}", diffs.len(), diffs.join("\n=====\n"));
}
