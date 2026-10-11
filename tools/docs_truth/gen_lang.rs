//! Generators fed by the compiler's own tables: keywords, operators (derived by lexing every
//! short punctuation string, so there is no hand-written operator list) and intrinsics.

use super::util::table;
use windjammer::intrinsics::Intrinsic;
use windjammer::lexer::keywords::{KeywordClass, KEYWORDS};
use windjammer::lexer::{Lexer, Token};

pub fn keywords() -> String {
    let rows = |class: KeywordClass| -> Vec<Vec<String>> {
        KEYWORDS.iter().filter(|k| k.class == class).map(|k| vec![format!("`{}`", k.word), k.summary.to_string()]).collect()
    };
    let mut s = String::from("# Keywords and operators\n\nGenerated from `src/lexer/keywords.rs` (pinned to the lexer by unit tests) and from the lexer itself.\n\n");
    for (title, class) in [
        ("Reserved words", KeywordClass::Keyword),
        ("Built-in type names", KeywordClass::BuiltinType),
        ("Literals and wildcard", KeywordClass::Literal),
        ("Soft keywords (identifiers outside their position)", KeywordClass::Soft),
    ] {
        s.push_str(&format!("## {title}\n\n{}\n", table(&["Word", "Meaning"], &rows(class))));
    }
    s.push_str("## Operators and punctuation\n\nEvery punctuation string of up to three characters that the lexer turns into exactly one token.\n\n");
    s.push_str(&table(&["Lexeme", "Token"], &operators()));
    s
}

/// (lexeme, token name) for every 1-3 char punctuation string that lexes to a single plain token.
pub fn operators() -> Vec<Vec<String>> {
    const CH: &[char] = &['+', '-', '*', '/', '%', '=', '!', '<', '>', '&', '|', '^', '~', '?', '.', ':', ',', ';', '@', '(', ')', '[', ']', '{', '}'];
    let mut cands: Vec<String> = CH.iter().map(|c| c.to_string()).collect();
    for a in CH {
        for b in CH {
            cands.push(format!("{a}{b}"));
            for c in CH {
                cands.push(format!("{a}{b}{c}"));
            }
        }
    }
    let mut rows: Vec<(String, String)> = Vec::new();
    // The lexer panics on characters it does not know (e.g. `~`); those are simply not operators.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for c in cands {
        let Ok(toks) = std::panic::catch_unwind(|| Lexer::new(&c).tokenize()) else { continue };
        if c.contains("//") || c.contains("/*") || toks.len() != 2 || toks[1] != Token::Eof {
            continue;
        }
        let name = format!("{:?}", toks[0]);
        if name.contains('(') || name.starts_with("Newline") {
            continue;
        }
        rows.push((c, name));
    }
    std::panic::set_hook(prev);
    rows.sort_by(|a, b| (a.0.len(), &a.0).cmp(&(b.0.len(), &b.0)));
    rows.into_iter().map(|(l, n)| vec![format!("`{l}`"), n]).collect()
}

pub fn intrinsics() -> String {
    let yn = |b: bool| if b { "yes" } else { "-" }.to_string();
    let rows: Vec<Vec<String>> = Intrinsic::ALL
        .iter()
        .map(|i| {
            vec![
                format!("`{}`", i.call_name()),
                format!("`{}`", i.surface_spelling()),
                yn(i.is_print()),
                yn(i.writes_to_stderr()),
                yn(i.appends_newline()),
                yn(i.diverges()),
                i.message_arg_index().map(|n| n.to_string()).unwrap_or_else(|| "-".into()),
            ]
        })
        .collect();
    format!(
        "# Intrinsics\n\nBuilt-in functions (ADR-0014 Phase 1), generated from `src/intrinsics/mod.rs`.\nThe canonical spelling is the plain call; the macro spelling is deprecated (`wj fix`).\n\n{}",
        table(&["Call", "Deprecated macro spelling", "Prints", "stderr", "Newline", "Diverges", "Message arg"], &rows)
    )
}
