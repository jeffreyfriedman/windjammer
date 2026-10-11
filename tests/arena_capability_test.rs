#![cfg(feature = "unstable_arenas")]
//! `std::arena` (ADR-0016): escape check (WJ0160) and interpreter semantics.
//! Programs in tests/arena_programs carry independent `// expect:` lines (computed by hand).

use windjammer::analyzer::arena_escape::{check_program, gate};
use windjammer::interpreter::error::ErrorKind;
use windjammer::interpreter::Interpreter;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};

fn parse(src: &str) -> Program<'static> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    Parser::new_with_source(tokens, "t.wj".into(), src.into()).parse().expect("parses")
}

fn errors(src: &str) -> Vec<String> {
    check_program(&parse(src)).into_iter().map(|e| format!("{}:{}", e.line, e.message)).collect()
}

fn assert_escape(src: &str, needle: &str) {
    let e = errors(src);
    assert!(e.iter().any(|m| m.contains(needle)), "expected an error containing {needle:?}, got {e:?}\n{src}");
}

fn assert_clean(src: &str) {
    assert_eq!(errors(src), Vec::<String>::new(), "{src}");
}

const P: &str = "use std::arena::{Arena, Scratch}\nstruct P { x: int }\n";

#[test]
fn returning_a_scratch_string_is_an_escape() {
    assert_escape(&format!("{P}fn f() -> string {{ let s = Scratch::new()\n let t = s.str(\"a\")\n return t }}"), "is returned");
    assert_escape(&format!("{P}fn f() -> string {{ let s = Scratch::new()\n s.str(\"a\") }}"), "final expression");
}

#[test]
fn returning_a_handle_of_a_local_arena_is_an_escape() {
    assert_escape(&format!("{P}fn f() {{ let mut a = Arena::new()\n let h = a.alloc(P {{ x: 1 }})\n return h }}"), "arena `a`");
}

#[test]
fn storing_into_an_outer_collection_or_variable_is_an_escape() {
    let push = format!("{P}fn f() {{ let mut keep = Vec::new()\n if true {{ let mut a = Arena::new()\n keep.push(a.alloc(P {{ x: 1 }})) }} }}");
    assert_escape(&push, "stored into `keep`");
    let assign = format!("{P}fn f() {{ let mut last = \"\"\n for i in 0..3 {{ let s = Scratch::new()\n last = s.str(\"x\") }} }}");
    assert_escape(&assign, "assigned to `last`");
    let field = format!("{P}struct W {{ names: Vec<string> }}\nimpl W {{ fn f(self) {{ let s = Scratch::new()\n self.names.push(s.str(\"x\")) }} }}");
    assert_escape(&field, "stored into `self`");
}

#[test]
fn block_value_cannot_carry_its_arena_out() {
    assert_escape(&format!("{P}fn f() {{ let t = {{ let s = Scratch::new()\n s.str(\"x\") }}\n println(t) }}"), "value of the block");
}

#[test]
fn use_after_reset_is_reported_but_try_get_is_allowed() {
    assert_escape(
        &format!("{P}fn f() {{ let mut a = Arena::new()\n let h = a.alloc(P {{ x: 1 }})\n a.reset()\n println(a.get(h).x) }}"),
        "used after `a.reset()`",
    );
    assert_clean(&format!("{P}fn f() {{ let mut a = Arena::new()\n let h = a.alloc(P {{ x: 1 }})\n a.reset()\n let o = a.try_get(h) }}"));
}

#[test]
fn copies_out_and_inner_collections_are_fine() {
    assert_clean(&format!(
        "{P}fn f() -> string {{ let s = Scratch::new()\n let mut parts = s.new_list()\n parts.push(s.str(\"a\"))\n let mut out = \"\"\n out.push_str(parts[0])\n let n = parts.len()\n return s.str(\"b\").to_string() }}"
    ));
    // Handles of an arena owned by the caller may leave the function.
    assert_clean(&format!("{P}fn spawn(pool: Arena<P>) -> Handle<P> {{ pool.alloc(P {{ x: 1 }}) }}"));
}

#[test]
fn shipped_programs_pass_the_check() {
    for name in ["particles", "scratch_labels"] {
        let src = std::fs::read_to_string(format!("tests/arena_programs/{name}.wj")).unwrap();
        assert_clean(&src);
    }
}

#[test]
fn programs_without_std_arena_skip_the_gate() {
    assert!(gate(&parse("fn main() { let s = 1 }")).is_ok());
}

fn interpret(src: &str) -> Result<String, (ErrorKind, String)> {
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let program = parse(&src);
            let mut it = Interpreter::new_capturing();
            match it.run_checked(&program) {
                Ok(_) => Ok(it.get_output()),
                Err(e) => Err((e.kind, e.message)),
            }
        })
        .unwrap()
        .join()
        .unwrap()
}

fn expected(src: &str) -> String {
    src.lines().filter_map(|l| l.strip_prefix("// expect: ")).map(|l| format!("{l}\n")).collect()
}

#[test]
fn interpreter_runs_the_arena_programs() {
    for name in ["particles", "scratch_labels"] {
        let src = std::fs::read_to_string(format!("tests/arena_programs/{name}.wj")).unwrap();
        assert_eq!(interpret(&src).map_err(|e| format!("{e:?}")), Ok(expected(&src)), "{name}");
    }
}

#[test]
fn interpreter_panics_on_stale_and_foreign_handles_like_a_debug_build() {
    // The static check cannot see through a helper function, so the dynamic check fires.
    let stale = format!("{P}fn read(a: Arena<P>, h: Handle<P>) -> int {{ a.get(h).x }}\nfn main() {{ let mut a = Arena::new()\n let h = a.alloc(P {{ x: 1 }})\n let mut hs = Vec::new()\n hs.push(h)\n a.reset()\n a.alloc(P {{ x: 2 }})\n println(read(a, hs[0])) }}");
    let got = interpret(&stale);
    assert!(matches!(&got, Err((ErrorKind::Panic, m)) if m.contains("stale arena handle")), "{got:?}");
    let foreign = format!("{P}fn main() {{ let mut a = Arena::new()\n let mut b = Arena::new()\n b.alloc(P {{ x: 0 }})\n let h = a.alloc(P {{ x: 1 }})\n println(b.get(h).x) }}");
    let got = interpret(&foreign);
    assert!(matches!(&got, Err((ErrorKind::Panic, m)) if m.contains("different arena")), "{got:?}");
}

#[test]
fn interpreter_rejects_escaping_programs_before_running() {
    let src = format!("{P}fn main() {{ let s = Scratch::new()\n println(\"ran\")\n let mut keep = Vec::new()\n if true {{ let t = Scratch::new()\n keep.push(t.str(\"x\")) }} }}");
    let got = interpret(&src);
    assert!(matches!(&got, Err((ErrorKind::Semantic, m)) if m.contains("WJ0160")), "{got:?}");
}
