//! Generated multi-module projects and the edits applied to them.

use super::support::Rng;

#[derive(Clone)]
pub struct Func {
    pub name: String,
    pub arity: usize,
    pub is_pub: bool,
}

#[derive(Clone)]
pub struct Module {
    pub funcs: Vec<Func>,
    /// Extra text appended verbatim (comments, syntax errors, numeric mixes).
    pub extra: String,
}

#[derive(Clone)]
pub struct Project {
    pub modules: Vec<Module>,
}

fn params(n: usize) -> String {
    (0..n).map(|k| format!("x{k}: int")).collect::<Vec<_>>().join(", ")
}

fn args(n: usize) -> String {
    (0..n).map(|k| format!("{}", k + 1)).collect::<Vec<_>>().join(", ")
}

impl Project {
    pub fn generate(rng: &mut Rng) -> Self {
        let n = 2 + rng.below(4);
        let modules = (0..n)
            .map(|_| Module {
                funcs: (0..1 + rng.below(3))
                    .map(|j| Func { name: format!("f{j}"), arity: 1 + rng.below(2), is_pub: true })
                    .collect(),
                extra: String::new(),
            })
            .collect();
        Project { modules }
    }

    /// The call-site arity is fixed at generation (`arity` of the original), so later arity edits break callers.
    pub fn render(&self, original: &Project) -> Vec<(String, String)> {
        let mut out = vec![("wj.toml".to_string(), "[package]\nname = \"gen\"\nversion = \"0.1.0\"\n".to_string())];
        for (i, m) in self.modules.iter().enumerate() {
            let mut t = String::new();
            if i > 0 {
                t.push_str(&format!("use crate::m{}\n\n", i - 1));
            }
            for f in &m.funcs {
                let call = match original.modules.get(i.wrapping_sub(1)).and_then(|p| p.funcs.first()) {
                    Some(g) if i > 0 => format!("m{}::{}({}) + x0", i - 1, g.name, args(g.arity)),
                    _ => "x0 + 1".to_string(),
                };
                let vis = if f.is_pub { "pub " } else { "" };
                t.push_str(&format!("{vis}fn {}({}) -> int {{\n    {call}\n}}\n\n", f.name, params(f.arity)));
            }
            t.push_str(&m.extra);
            out.push((format!("src/m{i}.wj"), t));
        }
        let mut main = String::new();
        for i in 0..self.modules.len() {
            main.push_str(&format!("use crate::m{i}\n"));
        }
        main.push_str("\nfn main() {\n    let mut t = 0\n");
        for (i, m) in original.modules.iter().enumerate() {
            for f in &m.funcs {
                main.push_str(&format!("    t = t + m{i}::{}({})\n", f.name, args(f.arity)));
            }
        }
        main.push_str("    println(\"${t}\")\n}\n");
        out.push(("src/main.wj".to_string(), main));
        out
    }

    /// One random edit; returns its description. Edits target dependency modules, never `main.wj`.
    pub fn edit(&mut self, original: &Project, rng: &mut Rng) -> String {
        let i = rng.below(self.modules.len());
        let m = &mut self.modules[i];
        let j = rng.below(m.funcs.len());
        match rng.below(9) {
            0 => {
                m.funcs[j].arity = 3 - m.funcs[j].arity;
                format!("m{i}::f{j} arity -> {}", m.funcs[j].arity)
            }
            1 => {
                m.funcs[j].is_pub = !m.funcs[j].is_pub;
                format!("m{i}::f{j} pub -> {}", m.funcs[j].is_pub)
            }
            2 => {
                m.funcs[j].name = format!("g{j}");
                format!("m{i}::f{j} renamed g{j}")
            }
            3 => {
                m.extra.push_str("fn broken( {\n");
                format!("m{i}: syntax error")
            }
            4 => {
                // Analysis phase (numeric inference, WJ0027): what the per-file memo stores.
                m.extra.push_str(&format!("pub fn wide{}(a: f32, b: f64) -> f64 {{\n    a + b\n}}\n", rng.below(1000)));
                format!("m{i}: f32 + f64")
            }
            5 => {
                m.extra.push_str(&format!("// edit {}\n", rng.next()));
                format!("m{i}: comment")
            }
            6 => {
                m.extra.push_str("pub fn uses_missing() -> int {\n    nowhere(1)\n}\n");
                format!("m{i}: unresolved call")
            }
            7 => {
                m.extra.clear();
                format!("m{i}: extras removed")
            }
            _ => {
                *m = original.modules[i].clone();
                format!("m{i}: reverted")
            }
        }
    }
}
