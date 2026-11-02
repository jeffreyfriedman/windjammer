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
use windjammer::parser::ArenaLedger;
use windjammer::parser::{Parser, Program};

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
    assert_eq!(
        ledger.freed(),
        0,
        "arenas freed while a Program still borrows them"
    );
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
    assert_eq!(
        strip_positions(&rendered),
        strip_positions(&format!("{again:?}"))
    );
    assert_eq!(churn.len(), 2000);
}

/// The arenas are freed exactly when the last program borrowing them goes away: not before (soundness),
/// not after (the LSP re-parses on every edit; see tests/parser_arena_memory_test.rs).
#[test]
fn arenas_are_freed_with_the_last_owner() {
    let (program, ledger) = parse_and_drop_parser("fn main() { let x = 1 + 2 }");
    let clone = program.clone(); // co-owner
    drop(program);
    assert_eq!(
        ledger.freed(),
        0,
        "a clone of the program still borrows the arenas"
    );
    assert_eq!(clone.items.len(), 1);
    drop(clone);
    assert_eq!(
        ledger.freed(),
        1,
        "last owner dropped: arenas must be freed"
    );
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
    let n = std::thread::spawn(move || format!("{program:?}").len())
        .join()
        .unwrap();
    assert!(n > 100);
    assert_eq!(
        ledger.freed(),
        1,
        "program (sole owner) dropped on the other thread"
    );
}

#[test]
fn one_parser_many_parses_have_independent_arenas() {
    let tokens = Lexer::new("fn a() {}").tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "t.wj".into(), "fn a() {}".into());
    let l1 = parser.arena_ledger();
    let p1 = parser.parse().unwrap();
    let l2 = parser.arena_ledger();
    assert!(
        !std::ptr::eq(l1.counter(), l2.counter()),
        "sealing installs a fresh bundle"
    );
    drop(p1);
    assert_eq!((l1.freed(), l2.freed()), (1, 0));
}

fn strip_positions(s: &str) -> String {
    s.split_whitespace()
        .filter(|w| !w.chars().any(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}

// The memory growth gate (precise, counting allocator, 1 MB per 1000 parses) lives in
// tests/parser_arena_memory_test.rs: it needs its own #[global_allocator], hence its own test binary.

/// Miri self-test (run explicitly: `cargo +nightly miri test --test parser_arena_soundness -- --ignored`):
/// the one hazard the owner model cannot rule out by type, an `Item` cloned out of a program and used after
/// every owner is gone. Miri must report a use after free here; if it does not, the miri run proves nothing.
#[test]
#[ignore = "deliberately unsound; only meaningful under miri, which must reject it"]
fn miri_detects_an_item_outliving_its_owner() {
    let (program, ledger) = parse_and_drop_parser(SRC);
    let item = program.items[1].clone(); // `impl P`: its method bodies live in the arenas
    drop(program);
    assert_eq!(ledger.freed(), 1);
    let _ = format!("{item:?}"); // reads freed arena memory
}

// ---- Gate: test helpers must not drop the Program while handing out references into its arenas ----

/// Function chunks (split at `fn ` lines) that return `&'static ...` yet call `parser.parse()` without leaking the Program.
fn early_owner_drops(src: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let mut chunks: Vec<&str> = Vec::new();
    let mut start = 0;
    for (i, _) in src.match_indices("\nfn ").chain(src.match_indices("\n    fn ")) {
        chunks.push(&src[start..i]);
        start = i;
    }
    chunks.push(&src[start..]);
    for c in chunks {
        let head = c.lines().find(|l| l.trim_start().starts_with("fn ")).unwrap_or("");
        let returns_static_ref = head.contains("-> &'static");
        let parses = c.contains("parser.parse()");
        let keeps_program = c.contains("Box::leak(Box::new(parser.parse()") || c.contains("Box::leak(Box::new(program") || c.contains("Program<'static> =");
        if returns_static_ref && parses && !keeps_program {
            bad.push(head.trim().to_string());
        }
    }
    bad
}

#[test]
fn helpers_returning_static_refs_keep_the_program_alive() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target" || n == "py_ports") {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    walk(&root.join("tests"), &mut files);
    walk(&root.join("src"), &mut files);
    let mut offenders = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if f.ends_with("parser_arena_soundness_test.rs") {
            continue; // holds the detector's own bad-example strings
        }
        for h in early_owner_drops(&text) {
            offenders.push(format!("{}: {h}", f.strip_prefix(&root).unwrap_or(&f).display()));
        }
    }
    assert!(offenders.is_empty(), "Program (owner of the arenas) dropped while references escape: {offenders:#?}");
}

#[test]
fn early_owner_drop_detector_can_fail() {
    let bad = "fn parse_expr(i: &str) -> &'static Expression<'static> {\n    let parser = Box::leak(Box::new(Parser::new(t)));\n    let program = parser.parse().unwrap();\n    program.items.first().unwrap()\n}\n";
    let good = "fn parse_expr(i: &str) -> &'static Expression<'static> {\n    let program = Box::leak(Box::new(parser.parse().unwrap()));\n    program.items.first().unwrap()\n}\n";
    assert_eq!(early_owner_drops(bad).len(), 1);
    assert!(early_owner_drops(good).is_empty());
}
