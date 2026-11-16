//! `use state_model` where no such module exists is an error at the `use` line (WJ0006) in `wj check` and `wj build`,
//! with a did-you-mean from the modules that do exist. It used to pass the check and fail later (or, interpreted,
//! decode into a same-named local type). Imports that resolve stay quiet: a module file beside the importer, a
//! module of the project, the standard library, a dependency declared in `wj.toml`, and a qualified crate import.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn run(args: &[&str], target: &Path) -> (bool, String) {
    let o = Command::new(WJ).args(args).arg(target).env("NO_COLOR", "1").output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

const SKILLS: &str = "use state_model\n\nfn main() {\n    println(\"hi\")\n}\n";

#[test]
fn a_bare_import_of_a_missing_module_is_an_error_at_the_use_line() {
    let d = Scratch::new("missing-module");
    write(d.path(), "skills.wj", SKILLS);
    let (ok, out) = run(&["check"], &d.join("skills.wj"));
    assert!(!ok && out.contains("error[WJ0006]") && out.contains("no module named `state_model`"), "{out}");
    assert!(out.contains("skills.wj:1"), "the error is at the use line: {out}");
}

#[test]
fn the_suggestion_comes_from_the_modules_that_exist() {
    let d = Scratch::new("missing-module-near");
    write(d.path(), "src/state_models.wj", "pub struct State {\n    pub n: int,\n}\n");
    write(d.path(), "src/skills.wj", "use state_model\n\npub fn f() -> int {\n    1\n}\n");
    write(d.path(), "wj.toml", "[package]\nname = \"p\"\nversion = \"0.1.0\"\n");
    let (ok, out) = run(&["check"], d.path());
    assert!(!ok && out.contains("`state_models`") && out.contains("similar name"), "{out}");
}

#[test]
fn imports_that_resolve_stay_quiet() {
    let d = Scratch::new("missing-module-quiet");
    write(d.path(), "state_model.wj", "pub struct State {\n    pub n: int,\n}\n");
    write(d.path(), "skills.wj", "use state_model\nuse std::strings\n\nfn main() {\n    println(\"hi\")\n}\n");
    let (ok, out) = run(&["check"], &d.join("skills.wj"));
    assert!(ok && !out.contains("WJ0006"), "a module file beside the importer: {out}");

    let d = Scratch::new("missing-module-dep");
    write(d.path(), "wj.toml", "[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n[dependencies]\nstrsim = \"0.11\"\n");
    write(d.path(), "src/main.wj", "use strsim\n\nfn main() {\n    println(\"hi\")\n}\n");
    let (_, out) = run(&["check"], d.path());
    assert!(!out.contains("WJ0006"), "a declared dependency: {out}");
}

#[test]
fn a_build_stops_on_it_before_generating_code() {
    let d = Scratch::new("missing-module-build");
    write(d.path(), "skills.wj", SKILLS);
    let out_dir = d.join("out");
    let o = Command::new(WJ)
        .args(["build"])
        .arg(d.join("skills.wj"))
        .args(["--target", "rust", "--no-cargo", "-o"])
        .arg(&out_dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(!o.status.success() && text.contains("WJ0006") && text.contains("state_model"), "{text}");
    assert!(!out_dir.join("skills.rs").exists(), "nothing is generated for a program that cannot resolve its imports");
}
