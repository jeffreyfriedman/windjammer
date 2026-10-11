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

//! Memory gate for repeated parsing (the LSP re-parses on every keystroke): the AST arenas must be freed
//! when the last `Program` that borrows them is dropped. The counting allocator is independent of the
//! parser's own bookkeeping. A mutation test re-introduces the old leak and requires the gate to fire.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size() as isize, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size() as isize, Ordering::Relaxed);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n as isize - l.size() as isize, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: Counting = Counting;

const SRC: &str = r#"
struct P { x: int, y: int }
impl P {
    fn sum(self) -> int {
        let a = self.x + self.y * 2
        match a { 0 => 1, n if n > 3 => { let t = n * (n + 1); t }, _ => if a > 1 { a } else { -a } }
    }
}
fn main() { let p = P { x: 1, y: 2 }; println("v = ${p.sum()}") }
"#;

fn parse_and_drop(leak: bool) {
    let tokens = Lexer::new(SRC).tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "t.wj".into(), SRC.into());
    let program = parser.parse().expect("parses");
    if leak {
        parser.leak_arenas_for_mutation_test(&program);
    }
    assert_eq!(program.items.len(), 3);
}

fn growth(leak: bool, n: usize) -> isize {
    for _ in 0..20 {
        parse_and_drop(leak); // warm up lazily initialised statics
    }
    let before = LIVE.load(Ordering::SeqCst);
    for _ in 0..n {
        parse_and_drop(leak);
    }
    LIVE.load(Ordering::SeqCst) - before
}

/// Both measurements live in one test: the counter is process-global and tests run on parallel threads.
#[test]
fn repeated_parses_do_not_grow_memory_and_the_gate_detects_a_leak() {
    const LIMIT: isize = 1024 * 1024; // 1 MB over 1000 parses
    let sound = growth(false, 1000);
    assert!(sound < LIMIT, "1000 parses grew live memory by {sound} bytes (limit {LIMIT}): arenas leak");
    let leaked = growth(true, 1000);
    assert!(
        leaked > LIMIT,
        "mutation: leaking the arenas must exceed the gate, grew only {leaked} bytes (gate is too loose)"
    );
    eprintln!("memory gate: sound growth {sound} B, leaked growth {leaked} B over 1000 parses");
}
