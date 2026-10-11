//! The documented keyword table. `docs/reference/keywords.md` is generated from it.
//!
//! The lexer's `match` in `identifier_lexing.rs` is the behaviour; this table is the
//! documentation source. `tests` below pin the two together in both directions (every table word
//! lexes as claimed; every `"word" =>` arm of the match appears here), so neither can drift.

/// How a reserved word lexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordClass {
    /// Reserved word with its own token.
    Keyword,
    /// Built-in type name with its own token.
    BuiltinType,
    /// `true` / `false`.
    Literal,
    /// Lexes as an identifier; only special in a syntactic position.
    Soft,
}

#[derive(Debug, Clone, Copy)]
pub struct KeywordInfo {
    pub word: &'static str,
    pub class: KeywordClass,
    pub summary: &'static str,
}

const fn k(word: &'static str, class: KeywordClass, summary: &'static str) -> KeywordInfo {
    KeywordInfo { word, class, summary }
}

use KeywordClass::{BuiltinType as T, Keyword as K, Literal as L, Soft as S};

pub const KEYWORDS: &[KeywordInfo] = &[
    k("fn", K, "function item or function type"),
    k("let", K, "local binding"),
    k("mut", K, "mutable binding or mutable reference"),
    k("const", K, "compile-time constant"),
    k("static", K, "static item"),
    k("struct", K, "struct item"),
    k("enum", K, "enum item"),
    k("trait", K, "trait item"),
    k("impl", K, "implementation block"),
    k("match", K, "pattern match expression"),
    k("if", K, "conditional expression"),
    k("else", K, "alternative branch of `if`"),
    k("for", K, "loop over an iterator"),
    k("in", K, "separator in `for pat in expr`"),
    k("while", K, "conditional loop"),
    k("loop", K, "unconditional loop"),
    k("return", K, "return from a function"),
    k("break", K, "leave a loop"),
    k("continue", K, "next loop iteration"),
    k("use", K, "import"),
    k("mod", K, "module declaration"),
    k("extern", K, "foreign declaration"),
    k("thread", K, "`thread { }` block"),
    k("async", K, "async function or block"),
    k("await", K, "await a future"),
    k("defer", K, "run a statement at scope exit"),
    k("pub", K, "public visibility"),
    k("self", K, "the receiver value"),
    k("unsafe", K, "unsafe block or function"),
    k("as", K, "type cast"),
    k("where", K, "trait-bound clause"),
    k("type", K, "type alias or associated type"),
    k("dyn", K, "trait object"),
    k("int", T, "default signed integer type"),
    k("int32", T, "32-bit signed integer"),
    k("uint", T, "default unsigned integer type"),
    k("float", T, "default floating-point type"),
    k("bool", T, "boolean type"),
    k("string", T, "owned string type"),
    k("true", L, "boolean literal"),
    k("false", L, "boolean literal"),
    k("_", L, "wildcard pattern"),
    k("Self", S, "the implementing type (lexes as an identifier)"),
    k("bound", S, "`bound Alias = Trait + ...` item (identifier elsewhere)"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{Lexer, Token};

    #[test]
    fn every_table_word_lexes_as_documented() {
        for kw in KEYWORDS {
            let toks = Lexer::new(kw.word).tokenize();
            let first = &toks[0];
            let is_ident = matches!(first, Token::Ident(_));
            assert_eq!(is_ident, kw.class == KeywordClass::Soft, "{}: lexed as {first:?}", kw.word);
        }
    }

    #[test]
    fn every_lexer_match_arm_is_in_the_table() {
        let src = include_str!("identifier_lexing.rs");
        let body = src.split("match ident.as_str()").nth(1).expect("keyword match");
        for line in body.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix('"') {
                if let Some((word, after)) = rest.split_once('"') {
                    if after.trim_start().starts_with("=>") {
                        assert!(KEYWORDS.iter().any(|k| k.word == word), "lexer arm {word:?} missing from KEYWORDS");
                    }
                }
            }
        }
    }
}
