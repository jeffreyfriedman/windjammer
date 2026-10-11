//! Gate 1: lexer span invariants (corpus + generated + mutation self-tests).

use crate::span_oracle::corpus::*;
use crate::span_oracle::lexer_oracle::*;
use crate::span_oracle::ratchet;
use std::collections::BTreeSet;

#[test]
fn hand_written_sources_satisfy_span_oracle() {
    for src in HAND_WRITTEN {
        let v = check_source(src);
        assert!(v.is_empty(), "source {src:?}:\n{}", v.join("\n"));
    }
}

#[test]
fn generated_sources_satisfy_span_oracle() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for case in 0..3000 {
        let n = 1 + rng.below(40);
        let src = generate(&mut rng, n);
        let v = check_source(&src);
        assert!(v.is_empty(), "generated case {case}, source {src:?}:\n{}", v.join("\n"));
    }
}

#[test]
fn repo_corpus_satisfies_span_oracle() {
    let files = repo_wj_files();
    assert!(files.len() > 500, "corpus suspiciously small: {}", files.len());
    let mut bad: BTreeSet<String> = BTreeSet::new();
    let mut hard_fail = Vec::new();
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let v = check_source(&src);
        if v.is_empty() {
            continue;
        }
        if v.len() == 1 && v[0].starts_with("lexer panicked") {
            bad.insert(rel(f));
        } else {
            hard_fail.push(format!("{}:\n  {}", rel(f), v.iter().take(5).cloned().collect::<Vec<_>>().join("\n  ")));
        }
    }
    assert!(hard_fail.is_empty(), "span violations:\n{}", hard_fail.join("\n"));
    ratchet::enforce("lexer_panics", &bad);
}

// ---- mutation self-tests: the oracle must reject every deliberately broken stream -------------

fn lexed(src: &str) -> Vec<windjammer::lexer::TokenWithLocation> {
    lex_all(src).expect("lexes")
}

const SAMPLE: &str = "fn é(x) {\r\n\tlet s = \"日本 ${x}\" // c\n\t/// doc\n\tx + 1u64\n}\n";

#[test]
fn oracle_accepts_unmutated_sample() {
    assert_eq!(check_stream(SAMPLE, &lexed(SAMPLE)), Vec::<String>::new());
}

fn mutant_is_rejected(name: &str, mutate: impl Fn(&mut Vec<windjammer::lexer::TokenWithLocation>)) {
    let mut t = lexed(SAMPLE);
    mutate(&mut t);
    let v = check_stream(SAMPLE, &t);
    assert!(!v.is_empty(), "mutation `{name}` was NOT detected by the lexer oracle");
}

#[test]
fn oracle_rejects_column_starting_at_leading_whitespace() {
    // The historical bug: the column pointed at the whitespace before the token.
    mutant_is_rejected("col-1", |t| t[3].column -= 1);
}
#[test]
fn oracle_rejects_column_off_by_one_high() {
    mutant_is_rejected("col+1", |t| t[2].column += 1);
}
#[test]
fn oracle_rejects_line_off_by_one() {
    mutant_is_rejected("line+1", |t| t[8].line += 1);
}
#[test]
fn oracle_rejects_byte_start_shift() {
    mutant_is_rejected("byte_start+1", |t| t[1].byte_start += 1);
}
#[test]
fn oracle_rejects_byte_end_short() {
    mutant_is_rejected("byte_end-1", |t| t[2].byte_end -= 1);
}
#[test]
fn oracle_rejects_end_column_wrong() {
    mutant_is_rejected("end_col+1", |t| t[4].end_column += 1);
}
#[test]
fn oracle_rejects_end_line_wrong() {
    mutant_is_rejected("end_line-1", |t| {
        let i = t.iter().position(|x| matches!(x.token, windjammer::lexer::Token::InterpolatedString(_))).unwrap();
        t[i].end_line += 1;
    });
}
#[test]
fn oracle_rejects_byte_range_in_middle_of_char() {
    mutant_is_rejected("mid-char", |t| t[1].byte_end += 1); // `é` is 2 bytes
}
#[test]
fn oracle_rejects_overlap() {
    mutant_is_rejected("overlap", |t| t[3].byte_start = t[2].byte_start);
}
#[test]
fn oracle_rejects_dropped_token() {
    mutant_is_rejected("dropped token", |t| {
        t.remove(5);
    });
}
#[test]
fn oracle_rejects_wrong_token_kind() {
    mutant_is_rejected("wrong kind", |t| t[0].token = windjammer::lexer::Token::Let);
}
#[test]
fn oracle_rejects_missing_eof() {
    mutant_is_rejected("no eof", |t| {
        t.pop();
    });
}
#[test]
fn oracle_rejects_char_vs_byte_column() {
    // A byte-based column would put the token after `é` one column later.
    mutant_is_rejected("byte column", |t| t[2].column += 1);
}

#[test]
fn trivia_check_rejects_hidden_tokens() {
    assert!(check_trivia("  \t ").is_ok());
    assert!(check_trivia("  // c").is_ok());
    assert!(check_trivia(" x ").is_err());
    assert!(check_trivia("/// doc").is_err());
    assert!(check_trivia("\n").is_err());
}
