#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! `Parser::parse()` returns `Program<'static>`; the nodes live in arenas the parser owns. Dropping the
//! parser first (the common pattern) must not invalidate the program. See docs/design/parser-arena-ownership.md.

use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::parser_impl::{Arena, ParserArenaLedger};

const SRC: &str = r#"
struct P { x: int, y: int }
impl P {
    fn sum(self) -> int {
        let a = self.x + self.y * 2
        match a {
            0 => 1,
            n if n > 3 => { let t = n * (n + 1); t }
            _ => if a > 1 { a } else { -a },
        }
    }
}
fn main() {
    let p = P { x: 1, y: 2 }
    let v = [1, 2, 3].len() + p.sum()
    println("v = ${v}")
}
"#;

fn parse_and_drop_parser(src: &str) -> (Program<'static>, ParserArenaLedger) {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "t.wj".into(), src.into());
    let ledger = parser.arena_ledger();
    let program = parser.parse().expect("parses");
    drop(parser); // the unsound pattern: the arenas' owner dies before the AST is used
    (program, ledger)
}

#[test]
fn program_outlives_its_parser() {
    let (program, ledger) = parse_and_drop_parser(SRC);
    assert_eq!(ledger.retired(), 3, "dropping a Parser must retire (not free) its three arenas");
    assert_eq!(ledger.freed(), 0, "an arena was freed while Program<'static> still borrows it");
    // Churn the allocator so that freed arena memory would be reused and clobbered.
    let churn: Vec<Vec<u8>> = (0..2000).map(|i| vec![(i % 251) as u8; 4096]).collect();
    let rendered = format!("{program:?}");
    assert!(rendered.contains("sum") && rendered.contains("println"), "AST is still intact: {}", &rendered[..200.min(rendered.len())]);
    // Deterministic across a second parse of the same text: a clobbered tree would differ.
    let (again, _) = parse_and_drop_parser(SRC);
    assert_eq!(strip_positions(&rendered), strip_positions(&format!("{again:?}")));
    assert_eq!(churn.len(), 2000);
}

#[test]
fn no_parser_ever_frees_its_arenas() {
    for _ in 0..20 {
        let (p, ledger) = parse_and_drop_parser("fn main() { let x = 1 + 2 }");
        assert_eq!(p.items.len(), 1);
        assert_eq!((ledger.retired(), ledger.freed()), (3, 0));
    }
}

/// Mutation self-test: the detector must fire when an arena is freed.
#[test]
fn ledger_detects_the_unsound_behavior() {
    let arena = Arena::<u32>::new_freeing_for_mutation_test();
    let ledger = arena.ledger();
    drop(arena);
    assert_eq!((ledger.freed(), ledger.retired()), (1, 0));
}

fn strip_positions(s: &str) -> String {
    s.split_whitespace().filter(|w| !w.chars().any(|c| c.is_ascii_digit())).collect::<Vec<_>>().join(" ")
}
