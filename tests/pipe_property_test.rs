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
//! Property test: a generated pipe chain parses to exactly the tree of (a) an oracle tree
//! written directly by the generator and (b) the hand-nested call source parsed by the same
//! parser. Covers precedence (lhs operators bind tighter than `|>`), left associativity,
//! every stage form and `?` / `.await` / `|>?` on stages.

#[path = "common/pipe_ast.rs"]
mod pipe_ast;
use pipe_ast::sexp;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// One generated case: the pipe source, hand-nested source, and the expected tree.
struct Case {
    pipe: String,
    nested: String,
    tree: String,
}

fn lhs(r: &mut Rng) -> (String, String) {
    match r.below(5) {
        0 => ("x".into(), "x".into()),
        1 => ("7".into(), "7".into()),
        2 => ("a + 1".into(), "(Add a 1)".into()),
        3 => ("a || b".into(), "(Or a b)".into()),
        _ => ("a * 2 + b".into(), "(Add (Mul a 2) b)".into()),
    }
}

fn gen(seed: u64) -> Case {
    let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let (src, tree) = lhs(&mut r);
    let (mut pipe, mut nested, mut tree) = (src.clone(), format!("({src})"), tree);
    let stages = 1 + r.below(6);
    for i in 0..stages {
        let f = format!("f{i}");
        let last = i + 1 == stages;
        let (op, wrap_try) = if r.below(4) == 0 { ("|>?", true) } else { ("|>", false) };
        // `|>?` unwraps the left-hand side first.
        let (cur_nested, cur_tree) = if wrap_try {
            (format!("({nested})?"), format!("(try {tree})"))
        } else {
            (nested.clone(), tree.clone())
        };
        let (stage, n, t) = match r.below(8) {
            0 => (f.clone(), format!("{f}({cur_nested})"), format!("(call {f} {cur_tree})")),
            1 => (format!("{f}(a)"), format!("{f}({cur_nested}, a)"), format!("(call {f} {cur_tree} a)")),
            2 => (format!("{f}(a, _, 3)"), format!("{f}(a, {cur_nested}, 3)"), format!("(call {f} a {cur_tree} 3)")),
            3 => (format!(".m{i}(a)"), format!("({cur_nested}).m{i}(a)"), format!("(meth {cur_tree} m{i} a)")),
            4 => (format!("{f}()?"), format!("{f}({cur_nested})?"), format!("(try (call {f} {cur_tree}))")),
            5 => (format!("{f}().await"), format!("{f}({cur_nested}).await"), format!("(await (call {f} {cur_tree}))")),
            6 => (format!("T::{f}(a, _)"), format!("T::{f}(a, {cur_nested})"), format!("(call T::{f} a {cur_tree})")),
            _ if last => (format!("|v| v + {i}"), format!("(|v| v + {i})({cur_nested})"), format!("(call (fn [v] (Add v {i})) {cur_tree})")),
            _ => (format!("(|v| v + {i})"), format!("(|v| v + {i})({cur_nested})"), format!("(call (fn [v] (Add v {i})) {cur_tree})")),
        };
        pipe = format!("{pipe} {op} {stage}");
        nested = n;
        tree = t;
    }
    Case { pipe, nested, tree }
}

#[test]
fn generated_pipe_chains_equal_manual_nesting_and_oracle_tree() {
    for seed in 1..=1500u64 {
        let c = gen(seed);
        let piped = sexp(&c.pipe).unwrap_or_else(|e| panic!("seed {seed}: `{}` failed: {e}", c.pipe));
        assert_eq!(piped, c.tree, "seed {seed}: pipe `{}` vs oracle", c.pipe);
        let manual = sexp(&c.nested).unwrap_or_else(|e| panic!("seed {seed}: `{}` failed: {e}", c.nested));
        assert_eq!(piped, manual, "seed {seed}: pipe `{}` vs nested `{}`", c.pipe, c.nested);
    }
}

#[test]
fn multi_line_layout_does_not_change_the_tree() {
    for seed in 1..=300u64 {
        let c = gen(seed);
        let broken = c.pipe.replace(" |>", "\n        |>");
        assert_eq!(sexp(&broken).unwrap(), sexp(&c.pipe).unwrap(), "seed {seed}");
        let commented = c.pipe.replace(" |>", " // c\n        |>");
        assert_eq!(sexp(&commented).unwrap(), sexp(&c.pipe).unwrap(), "seed {seed}");
    }
}

#[test]
fn generator_covers_every_stage_form() {
    let all: String = (1..=1500u64).map(|s| gen(s).pipe).collect::<Vec<_>>().join("\n");
    for needle in ["|>?", "(a, _, 3)", ".m", "()?", ".await", "T::f", "(|v|", "|> |v|"] {
        assert!(all.contains(needle), "generator never produced `{needle}`");
    }
}

/// Mutation self-test: a deliberately wrong oracle (right-associative nesting) must be caught.
#[test]
fn oracle_comparison_detects_wrong_associativity() {
    let piped = sexp("x |> f |> g").unwrap();
    assert_eq!(piped, "(call g (call f x))");
    assert_ne!(piped, "(call f (call g x))");
    assert_ne!(sexp("a + 1 |> f").unwrap(), "(Add a (call f 1))");
}

/// The formatter keeps every generated chain semantically identical and is idempotent, at the
/// default width and at a width that forces chain breaking.
#[test]
fn formatter_round_trips_generated_chains() {
    use windjammer::formatter::{format_source, FormatOptions};
    for width in [100usize, 30] {
        let opts = FormatOptions { max_width: width, indent: 4 };
        for seed in 1..=300u64 {
            let c = gen(seed);
            let src = format!("fn t() {{\n    let r = {}\n}}\n", c.pipe);
            let once = format_source(&src, &opts).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            assert_eq!(format_source(&once, &opts).unwrap(), once, "seed {seed} width {width}: not idempotent");
            let body = once.trim().trim_start_matches("fn t() {").trim_end_matches('}').trim().trim_start_matches("let r =");
            assert_eq!(sexp(body).unwrap(), c.tree, "seed {seed} width {width}: tree changed\n{once}");
        }
    }
}
