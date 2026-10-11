//! `docs/windjammer.ebnf` cross-checked against the real lexer and parser.
//!
//! The grammar is a reference for constrained-decoding harnesses and code-generating agents, so it must
//! not drift from `src/parser`. Three automated checks, none of which trusts the grammar:
//!
//! 1. *Structure*: it parses as EBNF, every referenced production is defined.
//! 2. *Vocabulary*: every quoted word/operator terminal lexes to exactly one real token (keywords against
//!    `src/lexer/keywords.rs`), and every reserved word of the lexer appears in the grammar or is listed
//!    in `[ebnf] unlisted_keywords` with a reason.
//! 3. *Derivation*: sentences are generated from the grammar (every alternative of every start production,
//!    plus an "everything optional present" variant) wrapped in the minimal context and handed to the real
//!    parser. A rejected sentence is a divergence; known divergences are listed in `[ebnf] divergences`
//!    (`"production#alt" = reason`). A listed divergence the parser now accepts fails, so the list only shrinks.

use super::config::Config;
use super::{Ctx, Finding};
use std::collections::BTreeMap;
use windjammer::lexer::keywords::{KeywordClass, KEYWORDS};
use windjammer::lexer::{Lexer, Token};

pub const PATH: &str = "docs/windjammer.ebnf";

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Alt(Vec<Node>),
    Seq(Vec<Node>),
    Opt(Box<Node>),
    Rep(Box<Node>),
    Term(String),
    Nt(String),
}

// ---------------------------------------------------------------------------------------------
// Parsing the EBNF text
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Id(String),
    Str(String),
    P(char),
}

fn lex(text: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = text.chars().collect();
    let (mut i, mut out) = (0, Vec::new());
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '(' && cs.get(i + 1) == Some(&'*') {
            // comments nest: `(* a (* b *) c *)`
            let (mut j, mut depth) = (i + 2, 1);
            while j < cs.len() && depth > 0 {
                if cs[j] == '(' && cs.get(j + 1) == Some(&'*') {
                    depth += 1;
                    j += 2;
                } else if cs[j] == '*' && cs.get(j + 1) == Some(&')') {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            if depth > 0 {
                return Err("unterminated comment".into());
            }
            i = j;
        } else if c == '"' || c == '\'' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < cs.len() && cs[j] != c {
                if cs[j] == '\\' && j + 1 < cs.len() {
                    j += 1;
                }
                s.push(cs[j]);
                j += 1;
            }
            if j >= cs.len() {
                return Err(format!("unterminated string starting {s:?}"));
            }
            out.push(Tok::Str(s));
            i = j + 1;
        } else if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < cs.len() && (cs[j].is_alphanumeric() || cs[j] == '_') {
                j += 1;
            }
            out.push(Tok::Id(cs[i..j].iter().collect()));
            i = j;
        } else {
            out.push(Tok::P(c));
            i += 1;
        }
    }
    Ok(out)
}

struct P {
    t: Vec<Tok>,
    i: usize,
}

impl P {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn alt(&mut self) -> Result<Node, String> {
        let mut alts = vec![self.seq()?];
        while self.peek() == Some(&Tok::P('|')) {
            self.i += 1;
            alts.push(self.seq()?);
        }
        Ok(if alts.len() == 1 {
            alts.pop().unwrap()
        } else {
            Node::Alt(alts)
        })
    }
    fn seq(&mut self) -> Result<Node, String> {
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            if matches!(
                t,
                Tok::P('|') | Tok::P(']') | Tok::P('}') | Tok::P(')') | Tok::P(';')
            ) {
                break;
            }
            items.push(self.atom()?);
        }
        Ok(if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Seq(items)
        })
    }
    fn atom(&mut self) -> Result<Node, String> {
        let t = self.t.get(self.i).cloned().ok_or("unexpected end")?;
        self.i += 1;
        let n = match t {
            Tok::Str(s) => Node::Term(s),
            Tok::Id(s) => Node::Nt(s),
            Tok::P('[') => self.close(']', Node::Opt as fn(Box<Node>) -> Node)?,
            Tok::P('{') => self.close('}', Node::Rep as fn(Box<Node>) -> Node)?,
            Tok::P('(') => {
                let n = self.alt()?;
                self.expect(')')?;
                n
            }
            other => return Err(format!("unexpected {other:?}")),
        };
        // lexical-only sugar: `a - b` (exception) and `"a"…"z"` (range) keep the left operand.
        while matches!(self.peek(), Some(Tok::P('-')) | Some(Tok::P('…'))) {
            self.i += 1;
            self.atom()?;
        }
        Ok(n)
    }
    fn close(&mut self, end: char, wrap: fn(Box<Node>) -> Node) -> Result<Node, String> {
        let n = self.alt()?;
        self.expect(end)?;
        Ok(wrap(Box::new(n)))
    }
    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.peek() == Some(&Tok::P(c)) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{c}`, found {:?}", self.peek()))
        }
    }
}

/// Production name -> body, in file order.
pub fn parse(text: &str) -> Result<Vec<(String, Node)>, String> {
    let mut p = P {
        t: lex(text)?,
        i: 0,
    };
    let mut out = Vec::new();
    while p.peek().is_some() {
        let Some(Tok::Id(name)) = p.peek().cloned() else {
            return Err(format!("expected a production name, found {:?}", p.peek()));
        };
        p.i += 1;
        p.expect('=').map_err(|e| format!("{name}: {e}"))?;
        let body = p.alt().map_err(|e| format!("{name}: {e}"))?;
        p.expect(';').map_err(|e| format!("{name}: {e}"))?;
        out.push((name, body));
    }
    Ok(out)
}

fn walk<'a>(n: &'a Node, f: &mut dyn FnMut(&'a Node)) {
    f(n);
    match n {
        Node::Alt(v) | Node::Seq(v) => v.iter().for_each(|c| walk(c, f)),
        Node::Opt(b) | Node::Rep(b) => walk(b, f),
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// Sentence generation
// ---------------------------------------------------------------------------------------------

/// Productions whose body is lexical (informative in the grammar): replaced by one sample token.
const LEXICAL: &[(&str, &str)] = &[
    ("IDENT", "@id"),
    ("INT_LIT", "1"),
    ("FLOAT_LIT", "1.5"),
    ("STRING_LIT", "\"s\""),
    ("CHAR_LIT", "'c'"),
    ("LIFETIME", "'a"),
];
/// Emitted after these productions so that statements/items sit on their own line (the parser uses ASI).
const LINE_END: &[&str] = &[
    "statement",
    "item",
    "field_decl",
    "match_arm",
    "let_stmt",
    "return_stmt",
    "break_stmt",
    "continue_stmt",
    "variant_decl",
    "function_sig",
    "type_alias",
];

pub struct Gen<'a> {
    prods: &'a BTreeMap<String, Node>,
    cost: BTreeMap<String, usize>,
    ids: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Min,
    Full,
}

impl<'a> Gen<'a> {
    pub fn new(prods: &'a BTreeMap<String, Node>) -> Gen<'a> {
        let mut g = Gen {
            prods,
            cost: BTreeMap::new(),
            ids: 0,
        };
        for _ in 0..prods.len() + 2 {
            for (name, body) in prods {
                let c = g.node_cost(body);
                if c < g.cost.get(name).copied().unwrap_or(usize::MAX) {
                    g.cost.insert(name.clone(), c);
                }
            }
        }
        g
    }
    fn node_cost(&self, n: &Node) -> usize {
        match n {
            Node::Term(_) => 1,
            Node::Nt(s) => self
                .cost
                .get(s)
                .copied()
                .unwrap_or(usize::MAX / 4)
                .saturating_add(0),
            Node::Opt(_) | Node::Rep(_) => 0,
            Node::Seq(v) => v
                .iter()
                .map(|c| self.node_cost(c))
                .fold(0usize, |a, b| a.saturating_add(b)),
            Node::Alt(v) => v.iter().map(|c| self.node_cost(c)).min().unwrap_or(0),
        }
    }
    fn emit(
        &mut self,
        n: &Node,
        force: Option<usize>,
        mode: Mode,
        depth: usize,
        out: &mut Vec<String>,
    ) {
        match n {
            Node::Term(s) => out.push(s.clone()),
            Node::Seq(v) => v.iter().for_each(|c| self.emit(c, None, mode, depth, out)),
            Node::Opt(b) => {
                if mode == Mode::Full && depth < 3 {
                    self.emit(b, None, mode, depth, out)
                }
            }
            Node::Rep(b) => {
                if mode == Mode::Full && depth < 3 {
                    self.emit(b, None, mode, depth, out)
                }
            }
            Node::Alt(v) => {
                let i = force.filter(|&i| i < v.len()).unwrap_or_else(|| {
                    (0..v.len())
                        .min_by_key(|&i| self.node_cost(&v[i]))
                        .unwrap_or(0)
                });
                self.emit(&v[i], None, mode, depth, out)
            }
            Node::Nt(name) => {
                if let Some((_, tok)) = LEXICAL.iter().find(|(n, _)| n == name) {
                    if *tok == "@id" {
                        self.ids += 1;
                        out.push(format!("v{}", self.ids));
                    } else {
                        out.push(tok.to_string());
                    }
                } else if let Some(body) = self.prods.get(name) {
                    // Full mode expands one level only: deeper uses are minimal (finite, readable sentences).
                    self.emit(
                        &body.clone(),
                        None,
                        if depth >= 1 { Mode::Min } else { mode },
                        depth + 1,
                        out,
                    );
                    if LINE_END.contains(&name.as_str()) {
                        out.push("\n".into());
                    }
                }
            }
        }
    }
    /// Sentences for production `name`: one per top-level alternative (`name#i`), plus `name#full`.
    pub fn sentences(&mut self, name: &str) -> Vec<(String, String)> {
        let Some(body) = self.prods.get(name).cloned() else {
            return vec![];
        };
        let n_alts = if let Node::Alt(v) = &body { v.len() } else { 1 };
        let mut out = Vec::new();
        for i in 0..n_alts {
            self.ids = 0;
            let mut t = Vec::new();
            self.emit(&body, Some(i), Mode::Min, 0, &mut t);
            out.push((format!("{name}#{i}"), join(&t)));
        }
        self.ids = 0;
        let mut t = Vec::new();
        self.emit(&body, None, Mode::Full, 0, &mut t);
        out.push((format!("{name}#full"), join(&t)));
        out
    }
}

fn join(toks: &[String]) -> String {
    // `@` attaches to the decorator name that follows it.
    toks.iter()
        .map(|t| {
            if t == "\n" {
                "\n".to_string()
            } else if t == "@" {
                "@".to_string()
            } else {
                format!("{t} ")
            }
        })
        .collect::<String>()
}

/// How a sentence of this production is embedded so the parser sees a complete program.
pub fn wrap(production: &str, s: &str) -> Option<String> {
    Some(match production {
        "item" | "use_decl" | "struct_decl" | "enum_decl" | "trait_decl" | "impl_block"
        | "function_decl" | "const_decl" | "type_alias" | "static_decl" | "mod_decl"
        | "extern_decl" | "bound_alias" | "program" => format!("{s}\n"),
        "statement" | "let_stmt" | "assignment_stmt" | "return_stmt" | "expression_stmt"
        | "if_stmt" | "if_let_stmt" | "match_stmt" | "for_stmt" | "while_stmt" | "loop_stmt"
        | "break_stmt" | "continue_stmt" | "defer_stmt" | "thread_stmt" | "async_stmt"
        | "block" => format!("fn main() {{\nloop {{\n{s}\n}}\n}}\n"),
        "expression" | "assignment_expr" | "or_expr" | "pipe_expr" | "and_expr" | "comparison"
        | "bitor" | "bitxor" | "bitand" | "shift" | "additive" | "multiplicative" | "unary"
        | "postfix" | "primary" | "if_expr" | "match_expr" | "closure" | "array_literal"
        | "struct_literal" | "literal" | "unsafe_block" => {
            format!("fn main() {{\nlet _r = {s}\n}}\n")
        }
        "type_expr" | "base_type" | "generic_type" | "array_type" | "tuple_type" | "fn_type"
        | "optional_type" => format!("fn f(a: {s}) {{\n}}\n"),
        "pattern" | "tuple_pattern" | "struct_pattern" | "enum_pattern" | "range_pattern" => {
            format!("fn main() {{\nmatch x {{\n{s} => 1\n}}\n}}\n")
        }
        _ => return None,
    })
}

/// Character-level (informative) productions: upper-case names and the helpers they use.
fn is_lexical(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
        || [
            "int_suffix",
            "float_suffix",
            "interpolation",
            "escape",
            "line_comment",
            "block_comment",
        ]
        .contains(&name)
}

pub fn parse_program(src: &str) -> Result<(), String> {
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            std::panic::catch_unwind(|| {
                let toks = Lexer::new(&src).tokenize_with_locations();
                let mut p = windjammer::parser::Parser::new_with_source(
                    toks,
                    "ebnf.wj".into(),
                    src.clone(),
                );
                p.parse().map(|_| ())
            })
            .unwrap_or_else(|_| Err("parser panicked".into()))
        })
        .map_err(|e| e.to_string())?
        .join()
        .unwrap_or_else(|_| Err("thread panicked".into()))
}

// ---------------------------------------------------------------------------------------------
// The check
// ---------------------------------------------------------------------------------------------

/// Words that appear quoted in the grammar but are not reserved lexer words (identifier-like names of
/// library types or soft keywords); everything else alphabetic must be a lexer keyword.
const NON_RESERVED_TERMINALS: &[&str] = &[
    "Option", "None", "crate", "bound", "thread", "defer", "Some",
];

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let text = ctx.read(PATH);
    if text.is_empty() {
        return vec![];
    }
    let f = |line: usize, m: String| Finding::fail("ebnf", PATH, line, m);
    let list = match parse(&text) {
        Ok(l) => l,
        Err(e) => return vec![f(1, format!("grammar does not parse as EBNF: {e}"))],
    };
    let mut out = Vec::new();
    let prods: BTreeMap<String, Node> = list.iter().cloned().collect();
    if prods.len() != list.len() {
        out.push(f(1, "a production is defined twice".into()));
    }
    // 1. structure
    let mut terms: Vec<String> = Vec::new();
    for (name, body) in &list {
        walk(body, &mut |n| match n {
            Node::Nt(r) if !prods.contains_key(r) => out.push(f(
                1,
                format!("`{name}` references undefined production `{r}`"),
            )),
            // Terminals of lexical productions (informative character-level rules) are not checked as tokens.
            Node::Term(t) if !is_lexical(name) => terms.push(t.clone()),
            _ => {}
        });
    }
    // 2. vocabulary
    terms.sort();
    terms.dedup();
    for t in &terms {
        if t.chars().all(|c| c.is_alphabetic() || c == '_') {
            if !KEYWORDS.iter().any(|k| k.word == t)
                && !NON_RESERVED_TERMINALS.contains(&t.as_str())
            {
                out.push(f(
                    1,
                    format!(
                        "terminal \"{t}\" is not a keyword of the lexer (src/lexer/keywords.rs)"
                    ),
                ));
            }
        } else if let Ok(toks) = std::panic::catch_unwind(|| Lexer::new(t).tokenize()) {
            if toks.len() != 2 && !t.contains(char::is_whitespace) {
                out.push(f(
                    1,
                    format!(
                        "terminal \"{t}\" does not lex as a single token ({} tokens)",
                        toks.len().saturating_sub(1)
                    ),
                ));
            }
        }
    }
    let unlisted = &cfg.ebnf.unlisted_keywords;
    for k in KEYWORDS
        .iter()
        .filter(|k| k.class == KeywordClass::Keyword || k.class == KeywordClass::Literal)
    {
        if !terms.iter().any(|t| t == k.word) && !unlisted.contains_key(k.word) {
            out.push(f(1, format!("lexer keyword `{}` never appears in the grammar (add it, or list it under [ebnf] unlisted_keywords with a reason)", k.word)));
        }
    }
    for k in unlisted.keys() {
        if terms.iter().any(|t| t == k) {
            out.push(f(1, format!("`{k}` is listed under [ebnf] unlisted_keywords but now appears in the grammar: remove the entry")));
        }
    }
    // 3. derivation
    let mut g = Gen::new(&prods);
    let mut seen = Vec::new();
    for (name, _) in &list {
        for (id, sentence) in g.sentences(name) {
            let Some(src) = wrap(name, &sentence) else {
                continue;
            };
            seen.push(id.clone());
            let res = parse_program(&src);
            match (res, cfg.ebnf.divergences.get(&id)) {
                (Err(e), None) => out.push(f(1, format!("grammar sentence `{id}` is rejected by the parser: {}\n    sentence: {}", e.lines().next().unwrap_or(""), sentence.trim().replace('\n', " ")))),
                (Ok(()), Some(why)) => out.push(f(1, format!("`{id}` is listed as a divergence ({why}) but the parser accepts it: remove the entry"))),
                _ => {}
            }
        }
    }
    for id in cfg.ebnf.divergences.keys() {
        if !seen.contains(id) {
            out.push(f(1, format!("[ebnf] divergences lists `{id}`, which is not a sentence of the grammar: remove the entry")));
        }
    }
    out
}

#[allow(dead_code)]
fn _unused(_: &Token) {}
