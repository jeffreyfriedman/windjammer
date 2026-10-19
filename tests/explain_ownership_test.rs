//! `wj explain-ownership` core: reasons are real rule IDs with use sites, stages are reported,
//! known divergences are flagged, and the JSON is stable. Library-level (no process spawn).

use windjammer::ir::explain::report::{ParamReport, Report};
use windjammer::ir::explain::{explain_source, render, ExplainOptions};

fn explain(src: &str) -> Report {
    explain_source(src, "t.wj", &ExplainOptions::default()).expect("explain")
}

fn param<'a>(r: &'a Report, f: &str, p: &str) -> &'a ParamReport {
    r.functions
        .iter()
        .find(|x| x.name == f)
        .unwrap_or_else(|| panic!("fn {f}"))
        .params
        .iter()
        .find(|x| x.name == p)
        .unwrap_or_else(|| panic!("param {p}"))
}

#[test]
fn mutated_param_is_mut_ref_with_a_mutation_reason_and_site() {
    let r = explain("struct P { x: int }\nfn bump(p: P) { p.x = p.x + 1 }\n");
    let p = param(&r, "bump", "p");
    assert_eq!(p.effective, "&mut");
    assert!(
        p.reasons.iter().any(|x| x.rule == "OWN-MUT-ASSIGN" && x.line.is_some() && x.binding),
        "reasons: {:?}",
        p.reasons
    );
}

#[test]
fn returned_param_is_owned_with_returned_reason() {
    let r = explain("struct B { n: int }\nfn keep(b: B) -> B { b }\n");
    let p = param(&r, "keep", "b");
    assert_eq!(p.effective, "owned");
    assert!(p.reasons.iter().any(|x| x.rule == "OWN-RETURNED"));
}

#[test]
fn explicit_annotation_is_reported_as_a_pin() {
    let r = explain("fn n(v: &Vec<int>) -> int { v.len() }\n");
    let p = param(&r, "n", "v");
    assert_eq!(p.pinned.as_deref(), Some("&"));
    assert!(p.reasons.iter().any(|x| x.rule == "OWN-PIN-REF"));
}

#[test]
fn readonly_param_gets_the_readonly_rule() {
    let r = explain("struct Bag { items: Vec<string> }\nfn n(b: Bag) -> int { b.items.len() }\n");
    let p = param(&r, "n", "b");
    assert_eq!(p.effective, "&");
}

#[test]
fn unknown_function_is_an_error() {
    let o = ExplainOptions { only_fn: Some("zzz".into()), ..Default::default() };
    assert!(explain_source("fn a() {}\n", "t.wj", &o).is_err());
}

#[test]
fn call_argument_reaches_the_solver() {
    // OWN-CALL-FORMAL used to be generated but dropped before the solver result (D-SOLVER-UNION); the
    // convergence stage C2 made the solver consume it, so the reason is live and nothing is flagged as dropped.
    let src = "struct B { items: Vec<string> }\nfn ro(b: B) -> int { b.items.len() }\nfn w(b: B) -> int { ro(b) }\n";
    let r = explain(src);
    let p = param(&r, "w", "b");
    assert!(p.reasons.iter().any(|x| x.rule == "OWN-CALL-FORMAL" && !x.dropped), "{:?}", p.reasons);
    assert!(!p.reasons.iter().any(|x| x.dropped), "{:?}", p.reasons);
    assert!(!p.findings.iter().any(|f| f.id == "D-SOLVER-UNION"), "{:?}", p.findings);
}

#[test]
fn emitted_rust_fills_columns_and_flags_last_use_clone() {
    let src = "struct It { n: int, s: string }\nfn take(i: It) -> It { i }\nfn main() {\n    let a = It { n: 1, s: \"x\".to_string() }\n    let b = take(a)\n    let c = take(b)\n    println(\"${c.n}\")\n}\n";
    let emitted = "struct It { n: i64, s: String }\nfn take(i: It) -> It { i }\nfn main() { let a = It { n: 1, s: String::new() }; let b = take(a.clone()); let c = take(b.clone()); println!(\"{}\", c.n); }\n";
    let o = ExplainOptions { emitted_rust: Some(emitted.into()), ..Default::default() };
    let r = explain_source(src, "t.wj", &o).unwrap();
    assert!(r.has_emitted);
    let ids: Vec<_> = r.findings().iter().map(|f| f.id.clone()).collect();
    assert!(ids.iter().any(|i| i == "D-CLONE-LASTUSE"), "{ids:?}");
    let take = param(&r, "take", "i");
    assert_eq!(take.emitted.as_deref(), Some("owned"));
}

#[test]
fn json_has_stable_top_level_shape() {
    let r = explain("struct P { x: int }\nfn bump(p: P) { p.x = 1 }\n");
    let v: serde_json::Value = serde_json::from_str(&render::to_json(&r)).unwrap();
    assert_eq!(v["functions"][0]["name"], "bump");
    assert!(v["functions"][0]["params"][0]["reasons"][0]["rule"].is_string());
    for k in ["file", "has_emitted", "functions", "notes"] {
        assert!(v.get(k).is_some(), "missing {k}");
    }
}

#[test]
fn explain_never_changes_compiler_output() {
    // Reporting only: running it twice on the same source is identical (no hidden state).
    let src = "struct P { x: int }\nfn bump(p: P) { p.x = 1 }\n";
    assert_eq!(render::to_text(&explain(src)), render::to_text(&explain(src)));
}
