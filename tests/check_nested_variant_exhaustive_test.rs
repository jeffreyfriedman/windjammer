//! WJ0010 counts a variant as covered when its payload patterns are variants of one enum that together cover it
//! (`Wrap::Ok(Inner::A(v))` and `Wrap::Ok(Inner::B)`), and still reports a variant whose payload patterns miss one.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn check(name: &str, arms: &str) -> (bool, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let src = format!(
        "enum Inner {{\n    A(int),\n    B,\n}}\nenum Wrap {{\n    Ok(Inner),\n    Err(string),\n}}\nfn f(r: Wrap) -> int {{\n    match r {{\n{arms}\n    }}\n}}\nfn main() {{\n    println(\"${{f(Wrap::Ok(Inner::B))}}\")\n}}\n"
    );
    let p = dir.path().join(format!("{name}.wj"));
    std::fs::write(&p, src).unwrap();
    let o = Command::new(WJ).args(["check", "--no-lint"]).arg(&p).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    (text.contains("WJ0010"), text)
}

#[test]
fn nested_variant_arms_that_cover_a_variant_are_exhaustive() {
    let (flagged, text) = check(
        "covered",
        "        Wrap::Ok(Inner::A(v)) => v,\n        Wrap::Ok(Inner::B) => 0,\n        Wrap::Err(_) => -1,",
    );
    assert!(!flagged, "{text}");
}

#[test]
fn a_nested_variant_arm_that_misses_a_variant_is_still_reported() {
    let (flagged, text) = check("missing", "        Wrap::Ok(Inner::A(v)) => v,\n        Wrap::Err(_) => -1,");
    assert!(flagged, "{text}");
}
