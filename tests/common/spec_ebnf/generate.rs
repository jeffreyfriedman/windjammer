use super::{Expr, Grammar};
use std::collections::HashMap;

pub const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "const", "static", "struct", "enum", "trait", "impl", "match", "if", "else",
    "for", "in", "while", "loop", "return", "break", "continue", "use", "mod", "extern", "thread",
    "async", "await", "defer", "pub", "self", "unsafe", "as", "where", "type", "dyn", "int", "int32",
    "uint", "float", "bool", "string", "true", "false", "_", "bound", "Self", "r", "None", "Some",
    "Ok", "Err", "Option", "Result", "Vec", "Box", "u8", "u16", "u32", "u64", "i8", "i16", "i32",
    "i64", "f32", "f64", "usize", "isize", "crate", "super", "move", "ref", "fn_", "println",
    "print", "panic", "assert", "debug", "todo", "unreachable", "format", "eprintln", "eprint",
    "assert_eq", "assert_ne", "main", "to_string", "wait", "ch", "spawn",
];

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

pub struct Gen<'g> {
    g: &'g Grammar,
    min: HashMap<String, u32>,
    pub rng: Rng,
    pub visited: std::collections::BTreeSet<String>,
}

const INF: u32 = u32::MAX / 4;

impl<'g> Gen<'g> {
    pub fn new(g: &'g Grammar, seed: u64) -> Self {
        let mut min: HashMap<String, u32> = g.prods.keys().map(|k| (k.clone(), INF)).collect();
        loop {
            let mut changed = false;
            for (n, p) in &g.prods {
                let m = size(p.body.as_ref().unwrap(), &min);
                if m < min[n] {
                    min.insert(n.clone(), m);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Gen { g, min, rng: Rng(seed | 1), visited: Default::default() }
    }

    /// Generate one sentence for production `name` (token list joined by spaces; lexical productions
    /// are concatenated without spaces).
    pub fn sentence(&mut self, name: &str, depth: u32) -> String {
        let mut out = String::new();
        self.prod(name, depth, false, &mut out);
        out
    }

    fn prod(&mut self, name: &str, depth: u32, lexical: bool, out: &mut String) {
        self.visited.insert(name.to_string());
        let p = &self.g.prods[name];
        let is_lex = lexical || name.chars().next().unwrap().is_ascii_uppercase();
        let body = p.body.as_ref().unwrap().clone();
        let reject = p.reject_keywords;
        let nl = p.nl;
        let mut text = String::new();
        for _ in 0..30 {
            text.clear();
            self.expr(&body, depth, is_lex, &mut text);
            if !reject || !KEYWORDS.contains(&text.as_str()) {
                break;
            }
        }
        out.push_str(&text);
        if is_lex && !lexical {
            out.push(' ');
        }
        if nl && !is_lex {
            out.push('\n');
        }
    }

    fn expr(&mut self, e: &Expr, depth: u32, lex: bool, out: &mut String) {
        let exhausted = depth == 0;
        match e {
            Expr::Seq(v) => v.iter().for_each(|x| self.expr(x, depth, lex, out)),
            Expr::Alt(v) => {
                let pick = if exhausted {
                    let best = v.iter().map(|a| size(a, &self.min)).min().unwrap();
                    let c: Vec<_> = v.iter().filter(|a| size(a, &self.min) == best).collect();
                    c[self.rng.below(c.len())]
                } else {
                    &v[self.rng.below(v.len())]
                };
                self.expr(&pick.clone(), depth, lex, out);
            }
            Expr::Opt(x) => {
                if !exhausted && self.rng.below(2) == 0 {
                    self.expr(x, depth, lex, out);
                }
            }
            Expr::Rep(x) => {
                let n = if exhausted { 0 } else { self.rng.below(3) };
                for _ in 0..n {
                    self.expr(x, depth, lex, out);
                }
            }
            Expr::Lit(s) => {
                out.push_str(s);
                if !lex {
                    out.push(' ');
                }
            }
            Expr::Class(r) => {
                let (a, b) = r[self.rng.below(r.len())];
                let span = b as u32 - a as u32 + 1;
                let c = char::from_u32(a as u32 + self.rng.below(span as usize) as u32).unwrap();
                out.push(c);
            }
            Expr::Ref(n) => {
                let d = if depth > 0 && self.g.prods[n].depth { depth - 1 } else { depth };
                // lexical sub-expansions never run out of depth: they are bounded by their own repetition
                let d = if lex { depth.max(1) } else { d };
                let child_lex = lex && n.chars().next().unwrap().is_ascii_uppercase();
                self.prod(n, d, child_lex, out);
            }
        }
    }
}

fn size(e: &Expr, min: &HashMap<String, u32>) -> u32 {
    match e {
        Expr::Seq(v) => v.iter().fold(0u32, |a, x| a.saturating_add(size(x, min)).min(INF)),
        Expr::Alt(v) => v.iter().map(|x| size(x, min)).min().unwrap_or(0),
        Expr::Opt(_) | Expr::Rep(_) => 0,
        Expr::Lit(_) | Expr::Class(_) => 1,
        Expr::Ref(n) => min[n],
    }
}
