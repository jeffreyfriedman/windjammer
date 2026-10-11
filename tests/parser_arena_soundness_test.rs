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
use windjammer::parser::ArenaLedger;

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

fn parse_and_drop_parser(src: &str) -> (Program<'static>, ArenaLedger) {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "t.wj".into(), src.into());
    let before_seal = parser.arena_ledger(); // the bundle this parse fills
    let program = parser.parse().expect("parses");
    drop(parser); // the formerly unsound pattern: the parser dies before the AST is used
    (program, before_seal)
}

#[test]
fn program_outlives_its_parser() {
    let (program, ledger) = parse_and_drop_parser(SRC);
    assert_eq!(ledger.freed(), 0, "arenas freed while a Program still borrows them");
    // Churn the allocator so that freed arena memory would be reused and clobbered.
    let churn: Vec<Vec<u8>> = (0..2000).map(|i| vec![(i % 251) as u8; 4096]).collect();
    let rendered = format!("{program:?}");
    assert!(
        rendered.contains("sum") && rendered.contains("println"),
        "AST is still intact: {}",
        &rendered[..200.min(rendered.len())]
    );
    // Deterministic across a second parse of the same text: a clobbered tree would differ.
    let (again, _) = parse_and_drop_parser(SRC);
    assert_eq!(strip_positions(&rendered), strip_positions(&format!("{again:?}")));
    assert_eq!(churn.len(), 2000);
}

/// The arenas are freed exactly when the last program borrowing them goes away: not before (soundness),
/// not after (the LSP re-parses on every edit; see tests/parser_arena_memory_test.rs).
#[test]
fn arenas_are_freed_with_the_last_owner() {
    let (program, ledger) = parse_and_drop_parser("fn main() { let x = 1 + 2 }");
    let clone = program.clone(); // co-owner
    drop(program);
    assert_eq!(ledger.freed(), 0, "a clone of the program still borrows the arenas");
    assert_eq!(clone.items.len(), 1);
    drop(clone);
    assert_eq!(ledger.freed(), 1, "last owner dropped: arenas must be freed");
}

/// A program assembled from another program's items (module merging) co-owns the storage.
#[test]
fn merged_program_keeps_storage_alive() {
    let (program, ledger) = parse_and_drop_parser(SRC);
    let merged = Program::from_items_of(program.items.clone(), [&program]);
    drop(program);
    assert_eq!(ledger.freed(), 0);
    assert!(format!("{merged:?}").contains("println"));
    drop(merged);
    assert_eq!(ledger.freed(), 1);
}

#[test]
fn programs_cross_threads() {
    let (program, ledger) = parse_and_drop_parser(SRC);
    let n = std::thread::spawn(move || format!("{program:?}").len()).join().unwrap();
    assert!(n > 100);
    assert_eq!(ledger.freed(), 1, "program (sole owner) dropped on the other thread");
}

#[test]
fn one_parser_many_parses_have_independent_arenas() {
    let tokens = Lexer::new("fn a() {}").tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "t.wj".into(), "fn a() {}".into());
    let l1 = parser.arena_ledger();
    let p1 = parser.parse().unwrap();
    let l2 = parser.arena_ledger();
    assert!(!std::ptr::eq(l1.counter(), l2.counter()), "sealing installs a fresh bundle");
    drop(p1);
    assert_eq!((l1.freed(), l2.freed()), (1, 0));
}

fn strip_positions(s: &str) -> String {
    s.split_whitespace()
        .filter(|w| !w.chars().any(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Resident set size of this process in MB (`ps`, so no extra dependencies); None where `ps` is unavailable.
fn rss_mb() -> Option<f64> {
    let o = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok().map(|kb| kb / 1024.0)
}

/// Growth gate for the never-freed arenas (docs/design/parser-arena-ownership.md): 1000 parses of a small file,
/// each dropped, must not grow the process by more than `MAX_GROWTH_MB`, so a long-lived process (the LSP)
/// cannot silently leak. Fails when per-parse retention rises. When the Arc-owned arenas land, tighten to ~1 MB.
#[test]
fn repeated_parses_stay_within_the_memory_growth_budget() {
    const MAX_GROWTH_MB: f64 = 16.0;
    if rss_mb().is_none() {
        return;
    }
    for _ in 0..50 {
        drop(parse_and_drop_parser(SRC));
    }
    let before = rss_mb().unwrap();
    for _ in 0..1000 {
        drop(parse_and_drop_parser(SRC));
    }
    let growth = rss_mb().unwrap() - before;
    eprintln!("1000 parses grew RSS by {growth:.1} MB (budget {MAX_GROWTH_MB} MB)");
    assert!(growth <= MAX_GROWTH_MB, "1000 parses of a {}-byte file grew RSS by {growth:.1} MB (budget {MAX_GROWTH_MB} MB): per-parse arena retention regressed", SRC.len());
}
