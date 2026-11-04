//! `wj check` rejects a call to a function a std module does not provide (`WJ0002`), and only that: real std
//! functions, aliases, local values that shadow a module name and modules the table does not cover stay quiet.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn check(src: &str) -> (bool, String) {
    let tmp = tempfile::TempDir::new().unwrap();
    let f = tmp.path().join("main.wj");
    std::fs::write(&f, src).unwrap();
    let o = Command::new(WJ).arg("check").arg(&f).env("NO_COLOR", "1").output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

#[test]
fn unknown_std_functions_are_errors_in_every_spelling() {
    for call in ["strings.index_of(s, \":\")", "strings::index_of(s, \":\")", "std::strings::index_of(s, \":\")"] {
        let (ok, out) = check(&format!("use std::strings\n\nfn main() {{\n    let s = \"a:b\"\n    println(\"${{{call}}}\")\n}}\n"));
        assert!(!ok && out.contains("error[WJ0002]") && out.contains("cannot find function `index_of` in `std::strings`"), "{call}: {out}");
        assert!(out.contains("main.wj:5"), "{call}: {out}");
    }
    let (ok, out) = check("use std::strings as st\n\nfn main() {\n    println(st.trimm(\"x\"))\n}\n");
    assert!(!ok && out.contains("`trimm` in `std::strings`") && out.contains("`strings::trim`"), "{out}");
}

#[test]
fn real_functions_shadowing_locals_and_uncovered_modules_are_quiet() {
    let src = "use std::strings\nuse std::path\n\nfn ends(path: string) -> bool {\n    path.ends_with(\".wj\")\n}\n\nfn main() {\n    let s = strings.trim(\"  a  \")\n    let log = [1, 2]\n    println(\"${s} ${log.len()} ${ends(\"x.wj\")}\")\n    let f = |strings: string| strings.len()\n    println(\"${f(\"ab\")}\")\n}\n";
    let (ok, out) = check(src);
    assert!(ok && !out.contains("WJ0002"), "{out}");
}
