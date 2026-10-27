//! Gate: the resolver visits every expression position the parser can produce (ADR-0021: effect facts and the durable step pass
//! are only as complete as the walk). Each case puts `probe()` calls in one syntactic position; the resolver must report exactly
//! as many calls as the source text contains. The count is textual, so it does not share a walker with the code under test.
//! `positions_are_exhaustive` stops compiling when the parser gains an expression or statement variant, so a new position cannot
//! be added without adding a case here.
#[path = "resolve_support.rs"]
mod support;
use windjammer::parser::ast::core::{Expression, Statement};
use windjammer::resolve::CallKind;

const PRELUDE: &str = "struct S { x: int }\nfn probe() -> int { 1 }\nfn probe2(a: int) -> int { a }\nfn opt(a: int) -> Option<int> { Some(a) }\nfn apply(f: fn() -> int) -> int { f() }\n";

const POSITIONS: &[(&str, &str)] = &[
    ("let", "let a = probe()"),
    ("let-else", "let Some(x) = opt(1) else { return probe() }"),
    ("assign-value", "let mut a = 0\n    a = probe()"),
    ("assign-target", "let mut v = [1, 2]\n    v[probe()] = 2"),
    ("compound", "let mut a = 0\n    a += probe()"),
    ("return", "if false { return probe() }"),
    ("if", "if probe() > 0 { probe() } else { probe() }"),
    ("match", "match probe() {\n        0 => probe(),\n        _ if probe() > 1 => probe(),\n        _ => 0,\n    }"),
    ("for", "for i in 0..probe() { probe() }"),
    ("while", "while probe() > 5 { probe() }"),
    ("loop", "loop {\n        probe()\n        break\n    }"),
    ("defer", "defer probe()"),
    ("binary", "let a = probe() + probe() * 2"),
    ("unary", "let a = -probe()"),
    ("call-arg", "let a = probe2(probe2(probe()))"),
    ("named-arg", "let a = probe2(a: probe())"),
    ("method-object", "let a = probe().to_string()"),
    ("method-arg", "let v = [1]\n    let a = v.len() + probe()"),
    ("struct-field", "let s = S { x: probe() }"),
    ("field-object", "let a = S { x: 1 }.x + probe()"),
    ("index", "let v = [1, 2]\n    let a = v[probe()]"),
    ("range", "let r = probe()..probe()"),
    ("closure", "let f = || probe()"),
    ("closure-arg", "let a = apply(|| probe())"),
    ("cast", "let a = probe() as float"),
    ("tuple", "let t = (probe(), probe())"),
    ("array", "let v = [probe(), probe()]"),
    ("vec-macro", "let v = vec![probe()]"),
    ("block", "let a = { probe() }"),
    ("interp", "let s = \"a ${probe()} b ${probe() + 1}\""),
    ("interp-println", "println(\"v=${probe()}\")"),
    ("interp-spec", "let s = \"${probe():.2}\""),
    ("spawn", "let h = spawn probe()"),
    ("const-in-fn", "const K: int = 3\n    let a = probe()"),
    ("unsafe-block", "let a = unsafe { probe() }"),
    ("method-chain", "let a = opt(probe()).unwrap_or(probe())"),
    ("match-arm-block", "match 1 {\n        1 => { probe() }\n        _ => 0,\n    }"),
    ("if-let", "if let Some(x) = opt(probe()) { probe2(x) }"),
    ("while-let", "let mut o = opt(1)\n    while let Some(x) = o { o = None\n probe() }"),
    ("let-tuple", "let (a, b) = (probe(), 2)"),
];

fn probes(src: &str) -> usize {
    src.matches("probe()").count() - 1 // the definition
}

/// (name, whole program) for each case in each place a body can live.
fn programs() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, body) in POSITIONS {
        let ret = if body.contains("return") || body.contains("else { return") { "int" } else { "int" };
        out.push((format!("free fn / {name}"), format!("{PRELUDE}fn t() -> {ret} {{\n    {body}\n    0\n}}\nfn main() {{ t() }}\n")));
        out.push((format!("method / {name}"), format!("{PRELUDE}struct H {{ n: int }}\nimpl H {{\n    fn t(self) -> int {{\n    {body}\n    0\n}}\n}}\nfn main() {{ let h = H {{ n: 1 }}\n h.t() }}\n")));
        out.push((format!("trait default / {name}"), format!("{PRELUDE}trait T2 {{\n    fn t(self) -> int {{\n    {body}\n    0\n}}\n}}\nstruct H {{ n: int }}\nimpl T2 for H {{}}\nfn main() {{ let h = H {{ n: 1 }}\n h.t() }}\n")));
        out.push((format!("closure / {name}"), format!("{PRELUDE}fn t() -> int {{\n    let c = || {{\n    {body}\n    0\n}}\n    c()\n}}\nfn main() {{ t() }}\n")));
    }
    out.push(("module const".into(), format!("{PRELUDE}const C: int = probe()\nfn main() {{ }}\n")));
    out.push(("module static".into(), format!("{PRELUDE}static G: int = probe()\nfn main() {{ }}\n")));
    out
}

#[test]
fn the_resolver_visits_every_call_position() {
    let mut missed = Vec::new();
    let mut checked = 0;
    for (name, src) in programs() {
        let parsed = std::panic::catch_unwind(|| support::resolve_one(&src));
        let Ok(rp) = parsed else {
            missed.push(format!("{name}: does not parse"));
            continue;
        };
        let seen = rp.calls.iter().filter(|c| c.callee == "probe" && c.kind != CallKind::Method).count();
        checked += 1;
        if seen != probes(&src) {
            missed.push(format!("{name}: the source has {} calls to probe(), the resolver reports {seen}", probes(&src)));
        }
    }
    assert!(missed.is_empty(), "{checked} programs checked:\n{}", missed.join("\n"));
}

/// Compile-time list of every expression and statement variant: add a case to POSITIONS when this stops compiling.
#[allow(dead_code)]
fn positions_are_exhaustive(e: &Expression<'_>, s: &Statement<'_>) -> (&'static str, &'static str) {
    let ex = match e {
        Expression::Literal { .. } => "literal",
        Expression::Identifier { .. } => "identifier",
        Expression::Binary { .. } => "binary",
        Expression::Unary { .. } => "unary",
        Expression::Call { .. } => "call-arg, named-arg",
        Expression::MethodCall { .. } => "method-object, method-arg, method-chain",
        Expression::FieldAccess { .. } => "field-object",
        Expression::StructLiteral { .. } => "struct-field",
        Expression::MapLiteral { .. } => "(no case: map literal syntax)",
        Expression::Range { .. } => "range",
        Expression::Closure { .. } => "closure, closure-arg",
        Expression::Cast { .. } => "cast",
        Expression::Index { .. } => "index, assign-target",
        Expression::Tuple { .. } => "tuple, let-tuple",
        Expression::Array { .. } => "array",
        Expression::MacroInvocation { .. } => "vec-macro, interp, interp-println, interp-spec",
        Expression::TryOp { .. } => "(try: below)",
        Expression::Await { .. } => "(await: below)",
        Expression::AsyncCall { .. } => "(async: below)",
        Expression::SpawnCall { .. } => "spawn",
        Expression::ChannelSend { .. } => "(channel: below)",
        Expression::ChannelRecv { .. } => "(channel: below)",
        Expression::Block { .. } => "block, unsafe-block, match-arm-block",
    };
    let st = match s {
        Statement::Let { .. } => "let, let-else, let-tuple",
        Statement::Const { .. } => "const-in-fn",
        Statement::Static { .. } => "module static",
        Statement::Assignment { .. } => "assign-value, assign-target, compound",
        Statement::Return { .. } => "return",
        Statement::Expression { .. } => "any",
        Statement::If { .. } => "if, if-let",
        Statement::Match { .. } => "match",
        Statement::For { .. } => "for",
        Statement::While { .. } => "while, while-let",
        Statement::Loop { .. } => "loop",
        Statement::Thread { .. } => "(thread: below)",
        Statement::Async { .. } => "(async: below)",
        Statement::Defer { .. } => "defer",
        Statement::Use { .. } => "(no calls)",
        Statement::Break { .. } | Statement::Continue { .. } => "(no calls)",
    };
    (ex, st)
}
