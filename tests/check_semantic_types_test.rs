//! `wj check` types for values bound by patterns and `?`/`unwrap`, fields called as methods, and unknown module paths: each
//! mistake is reported with a location, and the neighbouring valid programs are not.

use std::path::Path;
use windjammer::check::check_sources;

fn codes(src: &str) -> Vec<String> {
    check_sources(Path::new("."), &[("t.wj".to_string(), src.to_string())]).into_iter().filter(|d| d.is_error()).map(|d| d.code).collect()
}

const USER: &str = "struct User { login: string, id: int, tags: Vec<string> }\nfn get(login: string) -> Result<User, string> { Ok(User { login: login, id: 1, tags: [] }) }\nfn find() -> Option<User> { None }\n";

#[test]
fn a_field_the_value_bound_by_a_pattern_does_not_have() {
    for body in [
        "fn main() {\n    match get(\"a\") {\n        Ok(u) => println(\"${u.email}\"),\n        Err(e) => println(\"${e}\"),\n    }\n}\n",
        "fn main() {\n    match find() {\n        Some(u) => println(\"${u.email}\"),\n        None => {}\n    }\n}\n",
        "fn main() -> Result<int, string> {\n    let u = get(\"a\")?\n    println(\"${u.email}\")\n    Ok(1)\n}\n",
        "fn main() {\n    let u = get(\"a\").unwrap()\n    println(\"${u.email}\")\n}\n",
    ] {
        let c = codes(&format!("{USER}{body}"));
        assert!(c.contains(&"WJ0014".to_string()), "{body}\n=> {c:?}");
    }
}

#[test]
fn valid_uses_of_those_values_are_not_reported() {
    for body in [
        "fn main() {\n    match get(\"a\") {\n        Ok(u) => println(\"${u.login} ${u.id} ${u.tags.len()}\"),\n        Err(e) => println(\"${e}\"),\n    }\n}\n",
        "fn main() -> Result<int, string> {\n    let u = get(\"a\")?\n    println(\"${u.login}\")\n    Ok(u.id)\n}\n",
        // a name bound twice is not tracked
        "fn main() {\n    match get(\"a\") {\n        Ok(u) => println(\"${u.login}\"),\n        Err(e) => println(\"${e}\"),\n    }\n    let u = 5\n    println(\"${u}\")\n}\n",
    ] {
        assert!(codes(&format!("{USER}{body}")).is_empty(), "{body}");
    }
}

#[test]
fn a_field_is_not_a_method() {
    let c = codes(&format!("{USER}fn main() {{\n    let u = User {{ login: \"a\", id: 1, tags: [] }}\n    println(\"${{u.login()}}\")\n}}\n"));
    assert!(c.contains(&"WJ0015".to_string()), "{c:?}");
    // a field that may hold a function can be called
    let ok = codes("struct Hook { run: fn() -> int }\nfn one() -> int { 1 }\nfn main() {\n    let h = Hook { run: one }\n    println(\"${(h.run)()}\")\n}\n");
    assert!(ok.is_empty(), "{ok:?}");
}

#[test]
fn an_unknown_module_path_is_reported() {
    let c = codes("fn main() {\n    let r = github::list_issues()\n}\n");
    assert!(c.contains(&"WJ0002".to_string()), "{c:?}");
    for ok in [
        "use std::fs\nfn main() {\n    println(\"${fs::exists(\"a\")}\")\n}\n",
        "mod util {\n    pub fn one() -> int { 1 }\n}\nfn main() {\n    println(\"${util::one()}\")\n}\n",
        "fn main() {\n    println(\"${math::sqrt(4.0)}\")\n}\n",
        "fn main() {\n    let v = Vec::new()\n    println(\"${v.len()}\")\n}\n",
    ] {
        assert!(codes(ok).is_empty(), "{ok}\n=> {:?}", codes(ok));
    }
}

#[test]
fn a_crate_named_by_a_use_is_not_an_unknown_module() {
    let c = codes("use axum::Router\nfn main() {\n    axum::serve(1)\n}\n");
    assert!(!c.contains(&"WJ0002".to_string()), "{c:?}");
}
