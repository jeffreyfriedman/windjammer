//! A generated crate depends only on the default crates its code names (docs/economics C2, step 4): a program that
//! never names `windjammer_runtime`, `smallvec` or `serde` gets none of them, so its first `cargo build` compiles one
//! crate instead of the runtime's dependency tree (59 crates, about 30 s CPU, docs/economics/RESULTS.md).

use std::process::Command;

fn manifest_for(src: &str) -> String {
    let t = tempfile::tempdir().unwrap();
    let file = t.path().join("main.wj");
    std::fs::write(&file, src).unwrap();
    let out = t.path().join("gen");
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(&file)
        .output()
        .expect("run wj build");
    assert!(o.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&o.stderr));
    let manifest = std::fs::read_to_string(out.join("Cargo.toml")).unwrap();
    let code: String = std::fs::read_dir(&out)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    format!("{manifest}\n----- generated code -----\n{code}")
}

fn deps_of(manifest: &str) -> Vec<String> {
    let section = manifest.split("[dependencies]").nth(1).unwrap_or("");
    let section = section.split("\n[").next().unwrap_or("");
    section.lines().filter_map(|l| l.split('=').next()).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect()
}

#[test]
fn a_program_that_names_no_default_crate_has_no_dependencies() {
    let m = manifest_for("fn fib(n: int) -> int {\n    if n < 2 { return n }\n    fib(n - 1) + fib(n - 2)\n}\n\nfn main() {\n    println(\"${fib(20)}\")\n}\n");
    let code = m.split("----- generated code -----").nth(1).unwrap_or("");
    assert!(!code.contains("windjammer_runtime"), "precondition: the generated code names no runtime:\n{m}");
    assert_eq!(deps_of(&m), Vec::<String>::new(), "dependencies of a std-only program:\n{m}");
}

#[test]
fn a_program_that_uses_the_runtime_depends_on_it_and_nothing_else_by_default() {
    let m = manifest_for("use std::fs\n\nfn main() {\n    let text = fs::read_to_string(\"/dev/stdin\") ?? \"\"\n    println(\"${text.len()}\")\n}\n");
    let code = m.split("----- generated code -----").nth(1).unwrap_or("");
    assert!(code.contains("windjammer_runtime"), "precondition: the generated code names the runtime:\n{m}");
    assert_eq!(deps_of(&m), vec!["windjammer-runtime".to_string()], "{m}");
}

/// `[profile.release] lto` unset is automatic (docs/economics/RESULTS.md C4): off for a crate without other Windjammer
/// packages (fat LTO cost about 7x the incremental release CPU for about 1% run time there), on when the project depends
/// on one (cross-crate hot paths), and an explicit `lto = true` / `false` always wins.
fn project_manifest(wj_toml: &str, with_dep_package: bool) -> String {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("app");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("wj.toml"), wj_toml).unwrap();
    std::fs::write(root.join("src/main.wj"), "fn main() {\n    println(\"ok\")\n}\n").unwrap();
    if with_dep_package {
        let dep = t.path().join("util");
        std::fs::create_dir_all(dep.join("src")).unwrap();
        std::fs::write(dep.join("wj.toml"), "[package]\nname = \"util\"\nversion = \"0.1.0\"\n").unwrap();
        std::fs::write(dep.join("src/lib.wj"), "pub fn one() -> int {\n    1\n}\n").unwrap();
    }
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(root.join("build"))
        .arg(root.join("src"))
        .current_dir(&root)
        .output()
        .expect("run wj build");
    assert!(o.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&o.stderr));
    std::fs::read_to_string(root.join("build/Cargo.toml")).unwrap()
}

#[test]
fn release_lto_is_automatic_and_an_explicit_setting_wins() {
    let plain = "[package]\nname = \"app\"\nversion = \"0.1.0\"\n";
    assert!(project_manifest(plain, false).contains("lto = false"), "no Windjammer dependency: lto off");
    let with_dep = "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nutil = { path = \"../util\" }\n";
    assert!(project_manifest(with_dep, true).contains("lto = true"), "a Windjammer package dependency: lto on");
    let explicit = "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[profile.release]\nlto = true\n";
    assert!(project_manifest(explicit, false).contains("lto = true"), "explicit lto = true wins");
}
