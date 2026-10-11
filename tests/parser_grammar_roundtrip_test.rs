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

//! Grammar-based differential test: generate programs from the productions in
//! docs/windjammer.ebnf, then check three class-level properties:
//!
//!   G1  every generated program parses (a failure is a grammar hole in the parser or a bug in
//!       the generator's reading of the spec);
//!   G2  parse(format(p)) == parse(p) modulo spans (formatting, including forced line wrapping at
//!       a small width, never changes the AST: the newline-sensitivity regression class);
//!   G3  format is idempotent.
//!
//! Known divergences are listed in `KNOWN_*` below as a ratchet: the list may only shrink, and a
//! listed construct that starts passing fails the test until it is removed.

use windjammer::formatter::{format_source, FormatOptions};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
}

struct Gen {
    rng: Rng,
    /// Constructs the generator may use (lets the ratchet exclude known-bad ones by name).
    used: Vec<&'static str>,
}

const IDENTS: &[&str] = &["a", "b", "c", "n", "xs", "total", "item"];
const BINOPS: &[&str] = &["+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||", "&", "|", "^"];

impl Gen {
    fn note(&mut self, k: &'static str) {
        if !self.used.contains(&k) {
            self.used.push(k);
        }
    }

    fn ident(&mut self) -> String {
        self.rng.pick(IDENTS).to_string()
    }

    fn literal(&mut self) -> String {
        match self.rng.below(8) {
            0 => format!("{}", self.rng.below(1000)),
            1 => format!("{}.{}", self.rng.below(100), self.rng.below(100)),
            2 => "\"text\"".to_string(),
            3 => {
                self.note("interp");
                format!("\"v=${{{}}}\"", self.ident())
            }
            4 => {
                self.note("interp-string-first");
                "\"${\"abc\".len()}\"".to_string()
            }
            5 => "true".to_string(),
            6 => "'c'".to_string(),
            _ => format!("{}u64", self.rng.below(50)),
        }
    }

    fn args(&mut self, d: u32) -> String {
        let n = self.rng.below(3);
        (0..n).map(|_| self.expr(d)).collect::<Vec<_>>().join(", ")
    }

    fn closure(&mut self, d: u32) -> String {
        match self.rng.below(3) {
            0 => format!("|{}| {}", self.ident(), self.expr(d)),
            1 => format!("|{}, {}| {} + {}", self.ident(), self.ident(), self.ident(), self.ident()),
            _ => format!("|{}| {{ let t = {}\n t }}", self.ident(), self.expr(d)),
        }
    }

    fn expr(&mut self, d: u32) -> String {
        if d == 0 {
            return if self.rng.below(2) == 0 { self.literal() } else { self.ident() };
        }
        let d = d - 1;
        match self.rng.below(22) {
            0 | 1 => self.literal(),
            2 | 3 => self.ident(),
            4 | 5 => format!("{} {} {}", self.expr(d), self.rng.pick(BINOPS), self.expr(d)),
            6 => format!("-{}", self.atom(d)),
            7 => format!("!{}", self.atom(d)),
            8 => format!("{}({})", self.ident(), self.args(d)),
            9 => format!("{}.{}({})", self.atom(d), self.ident(), self.args(d)),
            10 => {
                self.note("closure-in-call");
                format!("{}.{}({})", self.atom(d), self.rng.pick(&["map", "filter", "for_each"]), self.closure(d))
            }
            11 => {
                self.note("closure-arg-among-args");
                format!("{}({}, {})", self.ident(), self.expr(d), self.closure(d))
            }
            12 => format!("{}.{}", self.atom(d), self.ident()),
            13 => format!("{}[{}]", self.atom(d), self.expr(d)),
            14 => format!("[{}]", self.args(d)),
            15 => format!("({}, {})", self.expr(d), self.expr(d)),
            16 => format!("{} as int", self.atom(d)),
            17 => format!("{}?", self.atom(d)),
            18 => {
                self.note("turbofish");
                format!("{}.{}::<int>({})", self.atom(d), self.rng.pick(&["parse", "collect", "into"]), self.args(d))
            }
            19 => {
                self.note("range-expr");
                format!("({}..{})", self.expr(d), self.expr(d))
            }
            20 => {
                self.note("pipe");
                format!("{} |> {}", self.atom(d), self.ident())
            }
            _ => format!("if {} {{ {} }} else {{ {} }}", self.expr(d), self.expr(d), self.expr(d)),
        }
    }

    /// An expression safe as a postfix/prefix operand.
    fn atom(&mut self, d: u32) -> String {
        match self.rng.below(3) {
            0 => self.ident(),
            1 => format!("({})", self.expr(d)),
            _ => format!("{}({})", self.ident(), self.args(d)),
        }
    }

    fn pattern(&mut self) -> String {
        match self.rng.below(10) {
            0 => "_".to_string(),
            1 => format!("{}", self.rng.below(10)),
            2 => {
                self.note("range-pattern");
                format!("{}..={}", self.rng.below(5), 5 + self.rng.below(5))
            }
            3 => {
                self.note("range-pattern");
                "'a'..='z'".to_string()
            }
            4 => {
                self.note("range-pattern");
                format!("{}..", 10 + self.rng.below(5))
            }
            5 => {
                self.note("range-pattern");
                format!("..={}", self.rng.below(5))
            }
            6 => format!("Some({})", self.ident()),
            7 => format!("{} | {}", self.rng.below(3), 3 + self.rng.below(3)),
            8 => format!("({}, _)", self.ident()),
            _ => self.ident(),
        }
    }

    fn block(&mut self, d: u32, in_loop: bool, ind: usize) -> String {
        let n = 1 + self.rng.below(3);
        let pad = " ".repeat(ind + 4);
        let mut s = String::from("{\n");
        for _ in 0..n {
            s.push_str(&pad);
            s.push_str(&self.stmt(d, in_loop, ind + 4));
            s.push('\n');
        }
        s.push_str(&" ".repeat(ind));
        s.push('}');
        s
    }

    fn stmt(&mut self, d: u32, in_loop: bool, ind: usize) -> String {
        let e = d.saturating_sub(1);
        match self.rng.below(16) {
            0 | 1 => format!("let {} = {}", self.ident(), self.expr(d)),
            2 => format!("let mut {} = {}", self.ident(), self.expr(d)),
            3 => format!("{} = {}", self.ident(), self.expr(d)),
            4 => format!("{} {}= {}", self.ident(), self.rng.pick(&["+", "-", "*", "/"]), self.expr(d)),
            5 => {
                self.note("deref-assign");
                format!("*{} = {}", self.ident(), self.expr(d))
            }
            6 => format!("{}({})", self.ident(), self.args(d)),
            7 => format!("if {} {}", self.expr(e), self.block(e, in_loop, ind)),
            8 => format!(
                "if {} {} else {}",
                self.expr(e),
                self.block(e, in_loop, ind),
                self.block(e, in_loop, ind)
            ),
            9 => format!("while {} {}", self.expr(e), self.block(e, true, ind)),
            10 => format!("for {} in {} {}", self.ident(), self.expr(e), self.block(e, true, ind)),
            11 => {
                let mut s = format!("match {} {{\n", self.expr(e));
                for _ in 0..1 + self.rng.below(3) {
                    s.push_str(&" ".repeat(ind + 4));
                    s.push_str(&format!("{} => {},\n", self.pattern(), self.expr(e)));
                }
                s.push_str(&" ".repeat(ind + 4));
                s.push_str(&format!("_ => {},\n", self.expr(e)));
                s.push_str(&" ".repeat(ind));
                s.push('}');
                s
            }
            12 => {
                self.note("labeled-loop");
                let l = self.rng.pick(&["outer", "rows", "a"]);
                let kind = self.rng.below(3);
                let head = match kind {
                    0 => format!("'{l}: loop"),
                    1 => format!("'{l}: while {}", self.expr(e)),
                    _ => format!("'{l}: for {} in {}", self.ident(), self.expr(e)),
                };
                let inner = format!(
                    "{{\n{p}if {} {{ {} '{l} }}\n{p}{}\n{q}}}",
                    self.expr(e),
                    if self.rng.below(2) == 0 { "break" } else { "continue" },
                    self.stmt(e, true, ind + 4),
                    p = " ".repeat(ind + 4),
                    q = " ".repeat(ind)
                );
                format!("{head} {inner}")
            }
            13 if in_loop => self.rng.pick(&["break", "continue"]).to_string(),
            14 => {
                self.note("loop-value");
                format!("let {} = loop {{ break {} }}", self.ident(), self.expr(e))
            }
            _ => format!("return {}", self.expr(d)),
        }
    }

    fn function(&mut self, i: usize) -> String {
        let d = 2 + self.rng.below(2) as u32;
        format!(
            "fn f{i}(a: int, b: int) -> int {}\n",
            self.block(d, false, 0)
        )
    }

    fn program(&mut self) -> String {
        let n = 1 + self.rng.below(3);
        (0..n).map(|i| self.function(i)).collect::<Vec<_>>().join("\n")
    }
}

fn fingerprint(src: &str) -> Result<String, String> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new_with_source(tokens, "g.wj".into(), src.into())));
    let program = parser.parse()?;
    Ok(strip_locations(&format!("{:?}", program.items)))
}

fn strip_locations(dbg: &str) -> String {
    let b = dbg.as_bytes();
    let mut out = String::with_capacity(dbg.len());
    let mut i = 0;
    while i < b.len() {
        if dbg[i..].starts_with("location: ") {
            i += "location: ".len();
            if dbg[i..].starts_with("None") {
                i += 4;
            } else if dbg[i..].starts_with("Some(") {
                let mut depth = 0;
                while i < b.len() {
                    match b[i] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            if dbg[i..].starts_with(", ") {
                i += 2;
            }
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

#[derive(Default, Debug)]
struct Report {
    parse_failures: Vec<(String, String)>,
    roundtrip_divergences: Vec<(String, String)>,
    not_idempotent: Vec<String>,
}

fn run(cases: u64, seed0: u64, widths: &[usize]) -> Report {
    let mut report = Report::default();
    for case in 0..cases {
        let mut g = Gen { rng: Rng(seed0 ^ (case.wrapping_mul(0x9E3779B97F4A7C15) | 1)), used: vec![] };
        let src = g.program();
        let original = match fingerprint(&src) {
            Ok(f) => f,
            Err(e) => {
                report.parse_failures.push((src, e));
                continue;
            }
        };
        for &w in widths {
            let opts = FormatOptions { max_width: w, indent: 4 };
            let formatted = match format_source(&src, &opts) {
                Ok(f) => f,
                Err(e) => {
                    report.roundtrip_divergences.push((src.clone(), format!("format error: {e}")));
                    continue;
                }
            };
            match fingerprint(&formatted) {
                Ok(f) if f == original => {}
                Ok(_) => report.roundtrip_divergences.push((
                    src.clone(),
                    format!("AST changed by formatting (width {w}):\n{formatted}"),
                )),
                Err(e) => report.roundtrip_divergences.push((
                    src.clone(),
                    format!("formatted program no longer parses (width {w}): {e}\n{formatted}"),
                )),
            }
            if let Ok(again) = format_source(&formatted, &opts) {
                if again != formatted {
                    report.not_idempotent.push(src.clone());
                }
            }
        }
    }
    report
}

fn big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e));
}

/// Ratchet: counts of known divergences on the fixed seed set below. These numbers may only go
/// down. When a fix lands, lower the constant (the test prints the current value).
const KNOWN_PARSE_FAILURES: usize = 0;
const KNOWN_ROUNDTRIP_DIVERGENCES: usize = 0;
const KNOWN_NOT_IDEMPOTENT: usize = 0;

#[test]
fn generated_programs_parse_and_survive_formatting() {
    big_stack(|| {
        let r = run(1200, 0x5EED_0001, &[100, 36]);
        let show = |v: &Vec<(String, String)>| {
            v.iter()
                .take(3)
                .map(|(s, e)| format!("--- program ---\n{s}\n--- problem ---\n{e}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        eprintln!(
            "grammar generator: parse_failures={} roundtrip={} non_idempotent={}",
            r.parse_failures.len(),
            r.roundtrip_divergences.len(),
            r.not_idempotent.len()
        );
        assert!(
            r.parse_failures.len() <= KNOWN_PARSE_FAILURES,
            "G1: {} generated programs rejected (ratchet {}):\n{}",
            r.parse_failures.len(),
            KNOWN_PARSE_FAILURES,
            show(&r.parse_failures)
        );
        assert!(
            r.roundtrip_divergences.len() <= KNOWN_ROUNDTRIP_DIVERGENCES,
            "G2: {} divergences (ratchet {}):\n{}",
            r.roundtrip_divergences.len(),
            KNOWN_ROUNDTRIP_DIVERGENCES,
            show(&r.roundtrip_divergences)
        );
        assert!(
            r.not_idempotent.len() <= KNOWN_NOT_IDEMPOTENT,
            "G3: {} non-idempotent (ratchet {}), e.g.\n{}",
            r.not_idempotent.len(),
            KNOWN_NOT_IDEMPOTENT,
            r.not_idempotent.first().cloned().unwrap_or_default()
        );
    });
}
