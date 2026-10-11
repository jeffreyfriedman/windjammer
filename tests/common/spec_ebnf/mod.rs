//! EBNF reader and sentence generator used by `spec_grammar_test.rs`.
//!
//! The grammar file is `docs/spec/grammar.ebnf`. Notation: `name = alt | alt ;`, `{ x }` repeat,
//! `[ x ]` optional, `( x )` group, `"lit"` terminal, `/[a-z_]/` character class (lexical
//! productions only), UPPERCASE names are lexical productions, lowercase names are syntactic.
//! Comments `(* ... *)` are ignored unless they start with `(*@`, which attach a directive to the
//! NEXT production: `@ctx <template with $$>`, `@nl`, `@status <parsed|unparsed|informative>`,
//! `@lex <TokenName>`, `@reject-keywords`, `@depth`.

pub mod generate;
pub mod parse;

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum Expr {
    Seq(Vec<Expr>),
    Alt(Vec<Expr>),
    Opt(Box<Expr>),
    Rep(Box<Expr>),
    Lit(String),
    Class(Vec<(char, char)>),
    Ref(String),
}

#[derive(Debug, Clone, Default)]
pub struct Production {
    pub name: String,
    pub body: Option<Expr>,
    pub ctx: Option<String>,
    pub nl: bool,
    pub status: String,
    pub lex: Option<String>,
    pub reject_keywords: bool,
    /// `@depth`: expanding this production consumes one level of the recursion budget.
    pub depth: bool,
}

#[derive(Debug, Default)]
pub struct Grammar {
    pub prods: BTreeMap<String, Production>,
    pub order: Vec<String>,
}
