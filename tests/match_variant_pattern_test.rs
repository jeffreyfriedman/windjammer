//! `wj check` rejects a `match` whose arms name a variant of the other wrapper than the scrutinee's declared type
//! (`Some` / `None` against a `Result`, `Ok` / `Err` against an `Option`), with the exact rewrite as a fix
//! (`src/check/match_variants.rs`). The scrutinee type must be declared (a std function's return, a project
//! function's return, a local bound once from one, a parameter type); nothing is guessed from a name.
//! Run: cargo test --test match_variant_pattern_test

use std::path::Path;
use windjammer::diagnostics::{apply, Diagnostic};

fn check(src: &str) -> Vec<Diagnostic> {
    windjammer::check::check_sources(Path::new("."), &[("main.wj".to_string(), src.to_string())])
}

fn wrong_pattern(d: &Diagnostic) -> bool {
    d.code == "WJ0003" && d.message.contains("pattern `")
}

fn fixed(src: &str) -> String {
    let diags = check(src);
    let edits = apply::collect_edits(&diags);
    apply::apply_edits(src, &edits["main.wj"]).unwrap()
}

/// The six-line program of the report: `env::var` returns a `Result`, the arms are `Option` patterns.
const OPTION_ARMS_ON_RESULT: &str = "use std::env\n\nfn main() {\n    match env::var(\"X\") {\n        Some(v) => println(\"${v}\"),\n        None => {},\n    }\n}\n";

#[test]
fn option_arms_on_a_result_are_an_error_with_the_exact_rewrite() {
    let diags = check(OPTION_ARMS_ON_RESULT);
    let hits: Vec<_> = diags.iter().filter(|d| wrong_pattern(d)).collect();
    assert_eq!(hits.len(), 2, "{diags:?}");
    assert!(hits[0].message.contains("expected `Result`, found `Option` pattern `Some(..)`"), "{}", hits[0].message);
    let sp = hits[0].primary_span.as_ref().expect("a span");
    assert_eq!(&OPTION_ARMS_ON_RESULT[sp.byte_start..sp.byte_end], "Some");
    assert!(hits[0].help.iter().any(|h| h.contains("`Ok(v)` and `Err(_)`")), "{:?}", hits[0].help);
    let out = fixed(OPTION_ARMS_ON_RESULT);
    assert!(out.contains("Ok(v) => println") && out.contains("Err(_) => {}"), "{out}");
    assert!(!check(&out).iter().any(wrong_pattern), "the rewrite is clean:\n{out}");
}

#[test]
fn result_arms_on_an_option_are_an_error() {
    let src = "use std::env\n\nfn main() {\n    match env::get(\"X\") {\n        Ok(v) => println(\"${v}\"),\n        Err(_) => {},\n    }\n}\n";
    let diags = check(src);
    assert_eq!(diags.iter().filter(|d| wrong_pattern(d)).count(), 2, "{diags:?}");
    let out = fixed(src);
    assert!(out.contains("Some(v) => println") && out.contains("None => {}"), "{out}");
    assert!(!check(&out).iter().any(wrong_pattern), "{out}");
}

#[test]
fn the_type_may_come_from_a_project_function_a_local_or_a_parameter() {
    let by_fn = "fn find(k: string) -> Result<string, string> {\n    Ok(k)\n}\n\nfn main() {\n    match find(\"a\") {\n        Some(v) => println(\"${v}\"),\n        None => {},\n    }\n}\n";
    assert_eq!(check(by_fn).iter().filter(|d| wrong_pattern(d)).count(), 2);
    let by_local = "use std::env\n\nfn main() {\n    let r = env::var(\"X\")\n    match r {\n        Some(v) => println(\"${v}\"),\n        None => {},\n    }\n}\n";
    assert_eq!(check(by_local).iter().filter(|d| wrong_pattern(d)).count(), 2);
    let by_param = "fn show(r: Option<string>) {\n    match r {\n        Ok(v) => println(\"${v}\"),\n        Err(_) => {},\n    }\n}\n\nfn main() {\n    show(None)\n}\n";
    assert_eq!(check(by_param).iter().filter(|d| wrong_pattern(d)).count(), 2);
}

#[test]
fn matching_patterns_and_unknown_types_are_not_reported() {
    // right patterns on both wrappers
    let right = "use std::env\n\nfn main() {\n    match env::var(\"X\") {\n        Ok(v) => println(\"${v}\"),\n        Err(_) => {},\n    }\n    match env::get(\"X\") {\n        Some(v) => println(\"${v}\"),\n        None => {},\n    }\n}\n";
    assert!(!check(right).iter().any(wrong_pattern));
    // a scrutinee whose type is not declared is not judged
    let unknown = "fn main() {\n    let items = vec![1, 2]\n    match items.get(0) {\n        Some(v) => println(\"${v}\"),\n        None => {},\n    }\n}\n";
    assert!(!check(unknown).iter().any(wrong_pattern));
    // an `Option` pattern inside an arm body of a correct `Result` match is not an arm pattern
    let nested = "use std::env\n\nfn main() {\n    match env::var(\"X\") {\n        Ok(v) => match env::get(\"Y\") {\n            Some(w) => println(\"${v}${w}\"),\n            None => {},\n        },\n        Err(_) => {},\n    }\n}\n";
    assert!(!check(nested).iter().any(wrong_pattern), "{:?}", check(nested));
    // a name bound twice is not judged
    let rebound = "use std::env\n\nfn main() {\n    let r = env::var(\"X\")\n    let r = env::get(\"X\")\n    match r {\n        Some(v) => println(\"${v}\"),\n        None => {},\n    }\n}\n";
    assert!(!check(rebound).iter().any(wrong_pattern));
    // a project enum with a variant of the same name makes the pattern ambiguous
    let own = "enum Maybe {\n    None,\n    One(int),\n}\n\nuse std::env\n\nfn main() {\n    match env::var(\"X\") {\n        None => {},\n        _ => {},\n    }\n}\n";
    assert!(!check(own).iter().any(wrong_pattern));
}

#[test]
fn an_error_binding_dropped_by_the_rewrite_is_not_applied_automatically() {
    let src = "use std::env\n\nfn main() {\n    match env::get(\"X\") {\n        Some(v) => println(\"${v}\"),\n        Err(e) => println(\"${e}\"),\n    }\n}\n";
    let diags = check(src);
    let err = diags.iter().find(|d| wrong_pattern(d) && d.message.contains("`Err(..)`")).expect("Err arm reported");
    assert_eq!(err.fixes[0].applicability, windjammer::diagnostics::Applicability::MaybeIncorrect);
}
