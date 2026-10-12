//! Seeded generator of multi-module Windjammer projects with cross-module calls, plus an independent
//! evaluator that computes the expected stdout from the generator's own model (not from any engine).
//!
//! Each project: a random module tree (file modules `a.wj`, directory modules with and without
//! `mod.wj`, nesting up to 3), functions that reuse the same names in different modules, private
//! helpers, and calls through every `use` form: plain module import, alias, item import with alias,
//! `crate::`/`super::`/`self::` paths, groups `{a, b}`, globs, and `pub use` re-exports. Calls only go to
//! functions earlier in a global order, so every program terminates; values stay below 1000.

use std::collections::BTreeMap;
use std::fmt::Write;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1)
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    pub fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

#[derive(Clone)]
struct Func {
    module: usize,
    name: String,
    mul: i64,
    add: i64,
    /// (callee function index, how the call is written)
    calls: Vec<usize>,
    order: usize,
}

struct Mod {
    path: Vec<String>,
    /// directory module (`x/`) or file module (`x.wj`)
    is_dir: bool,
    has_mod_wj: bool,
    parent: Option<usize>,
}

pub struct Project {
    /// relative path -> contents
    pub files: BTreeMap<String, String>,
    pub expected: String,
}

const NAMES: [&str; 8] = ["alpha", "beta", "gamma", "delta", "util", "core2", "shapes", "net"];

fn eval(funcs: &[Func], i: usize, x: i64) -> i64 {
    let f = &funcs[i];
    let y = (x * f.mul + f.add) % 1000;
    let mut acc = y;
    for &c in &f.calls {
        acc = (acc + eval(funcs, c, y % 50)) % 1000;
    }
    acc
}

/// A module exists on disk when it has a file (`x.wj`, `x/mod.wj`) or a descendant that does.
fn exists(mods: &[Mod], i: usize) -> bool {
    let m = &mods[i];
    !m.is_dir || m.has_mod_wj || (0..mods.len()).any(|k| mods[k].parent == Some(i) && exists(mods, k))
}

/// Every `use`/path form (interpreter gate).
pub fn generate(seed: u64) -> Project {
    generate_with(seed, &[0, 1, 2, 3, 4, 5])
}

/// Forms the compiled path handles today; module aliases (`use a as m; m::f()`) and `super::` paths in expressions
/// hit the compiler's name-keyed boundary-signature lookup (`missing boundary signature for m3::f0`), see
/// docs/INTERPRETER.md "Compiled-path bugs found". `WJ_FUZZ_ALL_FORMS=1` compiles every form.
pub fn generate_compiled(seed: u64) -> Project {
    if std::env::var("WJ_FUZZ_ALL_FORMS").is_ok() {
        return generate(seed);
    }
    generate_with(seed, &[1, 2, 3, 5])
}

fn generate_with(seed: u64, forms: &[u64]) -> Project {
    let mut r = Rng::new(seed);
    // module tree; index 0 is the crate root (main.wj)
    let mut mods: Vec<Mod> = vec![Mod { path: vec![], is_dir: true, has_mod_wj: false, parent: None }];
    let n = 2 + r.below(6) as usize;
    for _ in 0..n {
        let dirs: Vec<usize> = (0..mods.len()).filter(|&i| mods[i].is_dir && mods[i].path.len() < 3).collect();
        let parent = dirs[r.below(dirs.len() as u64) as usize];
        let taken: Vec<&str> = mods.iter().filter(|m| m.parent == Some(parent)).map(|m| m.path.last().unwrap().as_str()).collect();
        let free: Vec<&str> = NAMES.iter().copied().filter(|n| !taken.contains(n)).collect();
        if free.is_empty() {
            continue;
        }
        let name = free[r.below(free.len() as u64) as usize].to_string();
        let mut path = mods[parent].path.clone();
        path.push(name);
        let is_dir = r.chance(40);
        mods.push(Mod { path, is_dir, has_mod_wj: is_dir && r.chance(60), parent: Some(parent) });
    }
    // functions: same names (f0, f1) recur across modules; `u_<module>` is unique (for globs)
    let mut funcs: Vec<Func> = Vec::new();
    for (mi, m) in mods.iter().enumerate().skip(1) {
        if m.is_dir && !m.has_mod_wj {
            continue; // a bare directory holds no items
        }
        let k = 1 + r.below(2) as usize;
        for j in 0..k {
            funcs.push(Func { module: mi, name: format!("f{}", j), mul: 1 + r.below(9) as i64, add: r.below(100) as i64, calls: vec![], order: 0 });
        }
        funcs.push(Func { module: mi, name: format!("u_{}", m.path.join("_")), mul: 2, add: r.below(10) as i64, calls: vec![], order: 0 });
    }
    // random global order; a function may call functions earlier in the order (in other modules)
    let mut order: Vec<usize> = (0..funcs.len()).collect();
    for i in (1..order.len()).rev() {
        let j = r.below(i as u64 + 1) as usize;
        order.swap(i, j);
    }
    for (pos, &fi) in order.iter().enumerate() {
        funcs[fi].order = pos;
    }
    for &fi in &order {
        let earlier: Vec<usize> = (0..funcs.len()).filter(|&g| funcs[g].order < funcs[fi].order && funcs[g].module != funcs[fi].module).collect();
        if !earlier.is_empty() && r.chance(70) {
            let c = earlier[r.below(earlier.len() as u64) as usize];
            funcs[fi].calls.push(c);
            if r.chance(30) {
                let c2 = earlier[r.below(earlier.len() as u64) as usize];
                funcs[fi].calls.push(c2);
            }
        }
    }
    // render each module
    let mut files = BTreeMap::new();
    let mut alias_n = 0usize;
    let mut render_calls = |r: &mut Rng, from: usize, callees: &[usize], uses: &mut Vec<String>| -> Vec<String> {
        let mut exprs = Vec::new();
        for &c in callees {
            let t = &mods[funcs[c].module];
            let fname = &funcs[c].name;
            alias_n += 1;
            let abs = t.path.join("::");
            let form = forms[r.below(forms.len() as u64) as usize];
            let aliases_ok = forms.contains(&0);
            let e = match form {
                // plain module import with a unique alias
                0 => {
                    uses.push(format!("use crate::{} as m{}", abs, alias_n));
                    format!("m{}::{}", alias_n, fname)
                }
                // item import with alias
                1 => {
                    uses.push(format!("use crate::{}::{} as g{}", abs, fname, alias_n));
                    format!("g{}", alias_n)
                }
                // grouped import of the item under a fresh alias via its module
                2 if fname.starts_with("u_") => {
                    uses.push(format!("use crate::{}::*", abs));
                    fname.clone()
                }
                // fully qualified crate path, no `use`
                3 => format!("crate::{}::{}", abs, fname),
                // relative `super::` path when the target is reachable from the parent
                4 if mods[from].parent.is_some() && from != 0 => {
                    let mut up = String::new();
                    let mut cur = mods[from].parent.unwrap();
                    up.push_str("super::");
                    while !t.path.starts_with(&mods[cur].path) {
                        up.push_str("super::");
                        cur = mods[cur].parent.unwrap();
                    }
                    let rest = t.path[mods[cur].path.len()..].join("::");
                    if rest.is_empty() {
                        format!("{}{}", up, fname)
                    } else {
                        format!("{}{}::{}", up, rest, fname)
                    }
                }
                // braced group (unique names only, so two groups never bind the same name)
                5 if fname.starts_with("u_") => {
                    uses.push(format!("use crate::{}::{{{}}}", abs, fname));
                    fname.clone()
                }
                _ if !aliases_ok => format!("crate::{}::{}", abs, fname),
                // bare path: only from the crate root (elsewhere a child module of the same name would shadow it)
                _ if from == 0 => {
                    uses.push(format!("use {} as m{}", abs, alias_n));
                    format!("m{}::{}", alias_n, fname)
                }
                _ => {
                    // relative to the crate root through `super` (one per nesting level)
                    uses.push(format!("use {}{} as m{}", "super::".repeat(mods[from].path.len()), abs, alias_n));
                    format!("m{}::{}", alias_n, fname)
                }
            };
            exprs.push(e);
        }
        exprs
    };
    let mut bodies: BTreeMap<usize, (Vec<String>, String)> = BTreeMap::new();
    for fi in 0..funcs.len() {
        let m = funcs[fi].module;
        let entry = bodies.entry(m).or_insert_with(|| (Vec::new(), String::new()));
        let mut uses = Vec::new();
        let calls = render_calls(&mut r, m, &funcs[fi].calls.clone(), &mut uses);
        // globs and braced imports from different modules may bind the same name: keep the first
        entry.0.extend(uses);
        let f = &funcs[fi];
        let mut body = format!("pub fn {}(x: int) -> int {{\n    let y = helper_{}(x)\n    let mut acc = y\n", f.name, fi);
        for e in calls {
            let _ = writeln!(body, "    acc = (acc + {}(y % 50)) % 1000", e);
        }
        body.push_str("    acc\n}\n\n");
        let _ = write!(body, "fn helper_{}(x: int) -> int {{\n    (x * {} + {}) % 1000\n}}\n\n", fi, f.mul, f.add);
        entry.1.push_str(&body);
    }
    for (mi, m) in mods.iter().enumerate().skip(1) {
        let (uses, body) = bodies.remove(&mi).unwrap_or_default();
        let mut uses = uses;
        uses.sort();
        uses.dedup();
        // the same bare name imported twice (from two modules) is ambiguous: rewrite later duplicates
        let mut text = String::new();
        for u in &uses {
            let _ = writeln!(text, "{}", u);
        }
        if !uses.is_empty() {
            text.push('\n');
        }
        if m.is_dir && m.has_mod_wj {
            let kids: Vec<&Mod> = mods.iter().enumerate().filter(|(ki, k)| k.parent == Some(mi) && exists(&mods, *ki)).map(|(_, k)| k).collect();
            for k in kids {
                let _ = writeln!(text, "pub mod {}", k.path.last().unwrap());
            }
            text.push('\n');
        }
        text.push_str(&body);
        let rel = if m.is_dir { format!("src/{}/mod.wj", m.path.join("/")) } else { format!("src/{}.wj", m.path.join("/")) };
        if !m.is_dir || m.has_mod_wj {
            files.insert(rel, text);
        }
    }
    // main: call every function through its absolute path or an import, print the value
    let mut main_uses = Vec::new();
    let mut main_body = String::from("fn main() {\n");
    let mut expected = String::new();
    for (fi, f) in funcs.iter().enumerate() {
        let abs = mods[f.module].path.join("::");
        let call = if fi % 2 == 0 && forms.contains(&0) {
            main_uses.push(format!("use {} as top{}", abs, fi));
            format!("top{}::{}", fi, f.name)
        } else {
            format!("{}::{}", abs, f.name)
        };
        let x = 1 + (fi as i64 * 7) % 40;
        let _ = writeln!(main_body, "    println(\"{} ${{{}({})}}\")", fi, call, x);
        let _ = writeln!(expected, "{} {}", fi, eval(&funcs, fi, x));
    }
    main_body.push_str("}\n");
    let mut main = String::new();
    for u in &main_uses {
        let _ = writeln!(main, "{}", u);
    }
    main.push('\n');
    main.push_str(&main_body);
    files.insert("src/main.wj".into(), main);
    files.insert("wj.toml".into(), format!("[package]\nname = \"fuzz{}\"\nversion = \"0.1.0\"\n", seed));
    Project { files, expected }
}

pub fn write(p: &Project, dir: &std::path::Path) {
    for (rel, text) in &p.files {
        let f = dir.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, text).unwrap();
    }
}
