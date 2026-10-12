//! Seeded random sentence generator over the EBNF tree of `ebnf.rs`, used for docs/spec/grammar.ebnf.
//!
//! The deterministic generator in `ebnf.rs` covers every alternative once at minimal size; this one
//! explores combinations (nested expressions, optional parts together). Lexical (UPPERCASE) productions
//! are expanded character by character and concatenated; syntactic tokens are joined by spaces.
//! `@depth` productions consume the recursion budget; at budget zero the cheapest alternative is taken.

use super::ebnf::Node;
use std::collections::{BTreeMap, BTreeSet};
use windjammer::lexer::keywords::KEYWORDS;

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

/// Per-production generation flags taken from the grammar's directives.
#[derive(Default, Clone)]
pub struct Flags {
    pub nl: bool,
    pub depth: bool,
    pub reject_keywords: bool,
}

pub struct RandGen<'a> {
    prods: &'a BTreeMap<String, Node>,
    flags: &'a BTreeMap<String, Flags>,
    min: BTreeMap<String, u32>,
    pub rng: Rng,
    pub visited: BTreeSet<String>,
    /// Terminals emitted for the current sentence; past `BUDGET` every choice is the cheapest one.
    emitted: usize,
}

const BUDGET: usize = 60;

const INF: u32 = u32::MAX / 4;

pub fn upper(n: &str) -> bool {
    n.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

impl<'a> RandGen<'a> {
    pub fn new(prods: &'a BTreeMap<String, Node>, flags: &'a BTreeMap<String, Flags>, seed: u64) -> Self {
        let mut min: BTreeMap<String, u32> = prods.keys().map(|k| (k.clone(), INF)).collect();
        loop {
            let mut changed = false;
            for (n, body) in prods {
                let m = size(body, &min);
                if m < min[n] {
                    min.insert(n.clone(), m);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        RandGen { prods, flags, min, rng: Rng(seed | 1), visited: BTreeSet::new(), emitted: 0 }
    }

    pub fn sentence(&mut self, name: &str, depth: u32) -> String {
        let mut out = String::new();
        self.emitted = 0;
        self.prod(name, depth, false, &mut out);
        out
    }

    fn prod(&mut self, name: &str, depth: u32, in_lexeme: bool, out: &mut String) {
        self.visited.insert(name.to_string());
        let body = self.prods[name].clone();
        let f = self.flags.get(name).cloned().unwrap_or_default();
        let lexical = in_lexeme || upper(name);
        let mut text = String::new();
        for _ in 0..30 {
            text.clear();
            self.node(&body, depth, lexical, &mut text);
            if !f.reject_keywords || !KEYWORDS.iter().any(|k| k.word == text) {
                break;
            }
        }
        out.push_str(&text);
        if lexical && !in_lexeme {
            out.push(' ');
        }
        if f.nl && !lexical {
            out.push('\n');
        }
    }

    fn node(&mut self, n: &Node, depth: u32, lex: bool, out: &mut String) {
        let over = self.emitted > BUDGET;
        let exhausted = depth == 0 || over;
        match n {
            Node::Seq(v) => v.iter().for_each(|x| self.node(x, depth, lex, out)),
            Node::Alt(v) => {
                let pick = if exhausted {
                    let best = v.iter().map(|a| size(a, &self.min)).min().unwrap_or(0);
                    let c: Vec<&Node> = v.iter().filter(|a| size(a, &self.min) == best).collect();
                    c[self.rng.below(c.len())].clone()
                } else {
                    v[self.rng.below(v.len())].clone()
                };
                self.node(&pick, depth, lex, out);
            }
            Node::Opt(x) => {
                if !exhausted && self.rng.below(2) == 0 {
                    self.node(x, depth, lex, out);
                }
            }
            Node::Rep(x) => {
                // 0, 1 or 2 repetitions with probability 1/2, 1/3, 1/6: keeps sentences readable
                let k = if exhausted { 0 } else { [0, 0, 0, 1, 1, 2][self.rng.below(6)] };
                for _ in 0..k {
                    self.node(x, depth, lex, out);
                }
            }
            Node::Term(s) => {
                self.emitted += 1;
                out.push_str(s);
                if !lex {
                    out.push(' ');
                }
            }
            Node::Class(r) => {
                let (a, b) = r[self.rng.below(r.len())];
                let span = (b as u32 - a as u32 + 1) as usize;
                out.push(char::from_u32(a as u32 + self.rng.below(span) as u32).unwrap_or(a));
            }
            Node::Nt(name) => {
                let consumes = self.flags.get(name).is_some_and(|f| f.depth);
                let d = if consumes && depth > 0 { depth - 1 } else { depth };
                // inside a lexeme the budget never runs out (lexemes are bounded by their repetitions)
                let d = if lex && !over { depth.max(1) } else { d };
                self.prod(name, d, lex && upper(name), out);
            }
        }
    }
}

fn size(n: &Node, min: &BTreeMap<String, u32>) -> u32 {
    match n {
        Node::Seq(v) => v.iter().fold(0u32, |a, x| a.saturating_add(size(x, min)).min(INF)),
        Node::Alt(v) => v.iter().map(|x| size(x, min)).min().unwrap_or(0),
        Node::Opt(_) | Node::Rep(_) => 0,
        Node::Term(_) | Node::Class(_) => 1,
        Node::Nt(s) => min.get(s).copied().unwrap_or(INF),
    }
}
