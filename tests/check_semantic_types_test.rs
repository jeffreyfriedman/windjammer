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

/// A statement `match` with a map `insert` arm next to a unit arm is rustc E0308 (WJ0003); the same shapes with
/// every arm of one type, a `Vec` receiver, a block arm or the match as a value are not reported.
#[test]
fn a_statement_match_whose_arms_disagree_and_its_valid_neighbours() {
    let head = "use std::collections::HashMap\nuse std::collections::HashSet\n";
    let bad = [
        "fn main() {\n    let mut m: HashMap<int, int> = HashMap::new()\n    for i in 0..3 {\n        match i % 2 {\n            0 => m.insert(i, i),\n            _ => println(\"odd\"),\n        }\n    }\n}\n",
        "fn main() {\n    let mut s = HashSet::new()\n    match 1 {\n        1 => s.insert(1),\n        _ => println(\"x\"),\n    }\n    println(\"${s.len()}\")\n}\n",
        "fn f(m: HashMap<int, int>) {\n    match 2 {\n        0 => m.remove(&0),\n        _ => println(\"y\"),\n    }\n}\nfn main() {\n}\n",
    ];
    for src in bad {
        let src = format!("{head}{src}");
        assert!(codes(&src).contains(&"WJ0003".to_string()), "not reported:\n{src}");
    }
    let good = [
        // every arm the map's value: the match is an `Option` in a `let`
        "fn main() {\n    let mut m: HashMap<int, int> = HashMap::new()\n    let old = match 1 {\n        0 => m.insert(1, 1),\n        _ => m.remove(&1),\n    }\n    println(\"${m.len()} ${old.is_some()}\")\n}\n",
        // a `Vec` insert is unit
        "fn main() {\n    let mut v: Vec<int> = Vec::new()\n    match 1 {\n        0 => v.insert(0, 1),\n        _ => println(\"z\"),\n    }\n    println(\"${v.len()}\")\n}\n",
        // the value discarded in a block arm
        "fn main() {\n    let mut m: HashMap<int, int> = HashMap::new()\n    match 1 {\n        0 => {\n            let _ = m.insert(1, 1)\n        }\n        _ => println(\"w\"),\n    }\n    println(\"${m.len()}\")\n}\n",
        // a name that is a map in one binding and something else in another is not judged
        "fn main() {\n    let mut m: HashMap<int, int> = HashMap::new()\n    m.insert(1, 1)\n    let mut m: Vec<int> = Vec::new()\n    match 1 {\n        0 => m.insert(0, 1),\n        _ => println(\"q\"),\n    }\n    println(\"${m.len()}\")\n}\n",
    ];
    for src in good {
        let src = format!("{head}{src}");
        assert!(!codes(&src).contains(&"WJ0003".to_string()), "false WJ0003:\n{src}\n{:?}", codes(&src));
    }
}
