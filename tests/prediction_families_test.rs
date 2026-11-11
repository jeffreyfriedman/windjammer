//! Drift families B (a reference expected: lookups, adapters) and C (`String` against `&str`) predicted from
//! generated Rust (`check::widths::clashes`, ADR-0033). Every positive case must be rejected by rustc with E0308 and
//! every negative case accepted (rustc is the oracle here, not the walker's own idea of Rust). The corpus precision
//! scan for all families is `tests/width_prediction_test.rs`.

use windjammer::check::widths::clashes;

fn rustc_error(rust: &str) -> Option<String> {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("main.rs");
    std::fs::write(&f, rust).unwrap();
    let o = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "bin", "--emit=metadata", "-A", "warnings", "--crate-name", "fam", "-o"])
        .arg(tmp.path().join("out.rmeta"))
        .arg(&f)
        .output()
        .expect("rustc on PATH");
    (!o.status.success()).then(|| String::from_utf8_lossy(&o.stderr).to_string())
}

const POSITIVE: &[(&str, &str)] = &[
    ("B: set lookup by value", "use std::collections::HashSet;\nfn main() { let mut s: HashSet<i64> = HashSet::new(); s.insert(1_i64); let b = s.contains(2_i64); }"),
    ("B: map lookup by value", "use std::collections::HashMap;\nfn main() { let m: HashMap<i64, i64> = HashMap::new(); let x = m.get(3_i64); }"),
    ("B: map key String by value", "use std::collections::HashMap;\nfn main() { let m: HashMap<String, i64> = HashMap::new(); let k = String::from(\"a\"); let x = m.contains_key(k); }"),
    ("B: vec contains by value", "fn main() { let v: Vec<i64> = vec![1_i64]; let x = v.contains(1_i64); }"),
    ("B: owned int for a reference parameter", "fn r(x: &i64) -> i64 { *x }\nfn main() { let y = 5_i64; r(y); }"),
    ("C: literal to a String closure parameter", "fn main() { let f = |who: String| who.len(); f(\"arg\"); }"),
    ("C: String + String", "fn main() { let out = String::new(); let pad = 2_usize; let s = out + \"0\".repeat(pad); }"),
    ("C: String to a &str parameter", "fn greet(n: &str) -> usize { n.len() }\nfn main() { let s = String::from(\"x\"); greet(s); }"),
    ("C: literal to a String parameter", "fn own(n: String) -> usize { n.len() }\nfn main() { own(\"x\"); }"),
    ("C: &str assigned to a String", "fn main() { let mut cur = String::new(); let w: &str = \"a\"; cur = w; }"),
    ("C: match arms String and &str", "fn main() { let c = true; let s = match c { true => String::from(\"a\"), false => \"b\" }; }"),
];

const NEGATIVE: &[(&str, &str)] = &[
    ("deref coercion &String to &str", "fn greet(n: &str) -> usize { n.len() }\nfn main() { let s = String::from(\"x\"); greet(&s); }"),
    ("map lookup by reference", "use std::collections::HashMap;\nfn main() { let m: HashMap<i64, i64> = HashMap::new(); let x = m.get(&3_i64); }"),
    ("String + &str", "fn main() { let a = String::from(\"a\"); let b = a + \"b\"; let c = b + &String::from(\"c\"); }"),
    ("Vec::get takes an index by value", "fn main() { let v = vec![1_i64]; let x = v.get(0_usize); }"),
    ("deref coercion &&i64 to &i64", "fn r(x: &i64) -> i64 { *x }\nfn main() { let y = 5_i64; let rr = &&y; r(rr); }"),
    ("to_string makes a String", "fn main() { let s: &str = \"a\"; let t: String = s.to_string(); }"),
    ("String += &str", "fn main() { let mut s = String::new(); s += \"x\"; s += &String::from(\"y\"); }"),
    ("&String for a &str binding", "fn main() { let x: &str = &String::from(\"a\"); }"),
    ("format! makes a String", "fn own(n: String) -> usize { n.len() }\nfn main() { own(format!(\"{}\", 1)); }"),
    ("a user type named like a std one", "struct HashSet;\nimpl HashSet { fn new() -> HashSet { HashSet } fn contains(&self, x: i64) -> bool { x > 0 } }\nfn main() { let s = HashSet::new(); let b = s.contains(2_i64); }"),
    ("an unannotated closure parameter", "fn main() { let f = |who| who; let x: &str = f(\"arg\"); }"),
];

#[test]
fn families_b_and_c_are_found_where_rustc_rejects() {
    for (why, rust) in POSITIVE {
        let found = clashes(rust);
        assert_eq!(found.len(), 1, "{why}: {found:?}");
        let err = rustc_error(rust).unwrap_or_else(|| panic!("{why}: rustc accepts the positive case"));
        assert!(err.contains("E0308"), "{why}: {err}");
    }
}

#[test]
fn valid_string_and_reference_shapes_are_not_reported() {
    for (why, rust) in NEGATIVE {
        let found = clashes(rust);
        assert!(found.is_empty(), "{why}: {found:?}");
        assert!(rustc_error(rust).is_none(), "{why}: the negative case must be valid Rust:\n{}", rustc_error(rust).unwrap());
    }
}
