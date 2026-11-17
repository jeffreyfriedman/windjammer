//! wj.lock (ADR-0010): whole-graph resolution in dependency order, content hashes, drift errors with the fixing command.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use windjammer::package::lock::{enforce, plan, render, update, Lock, Policy, LOCKFILE};



fn write(root: &Path, rel: &str, s: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, s).unwrap();
}

fn names(l: &Lock) -> Vec<&str> {
    l.packages.iter().map(|(n, _)| n.as_str()).collect()
}

/// app -> a -> b, app -> c; written in an order that is not the dependency order.
fn graph(r: &Path) {
    write(r, "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nc = { path = \"../c\" }\na = { path = \"../a\" }\n");
    write(r, "a/wj.toml", "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"../b\" }\n");
    write(r, "a/src/lib.wj", "pub fn fa() -> int { 1 }\n");
    write(r, "b/wj.toml", "[package]\nname = \"b\"\n");
    write(r, "b/src/lib.wj", "pub fn fb() -> int { 2 }\n");
    write(r, "c/wj.toml", "[package]\nname = \"c\"\n");
    write(r, "c/src/lib.wj", "pub fn fc() -> int { 3 }\n");
}

#[test]
fn packages_are_locked_dependencies_first_with_hashes() {
    let d = tempfile::tempdir().unwrap();
    graph(d.path());
    let app = d.path().join("app");
    let l = plan(&app, &BTreeMap::new(), Policy::Refresh).unwrap();
    assert_eq!(names(&l), ["b", "a", "c"], "b before a (a needs it); a before c only because a < c is not required: ties by name");
    let a = &l.packages[1].1;
    assert_eq!(a.dependencies, ["b"]);
    assert!(a.hash.starts_with("sha256:") && a.hash.len() == 7 + 64);
    assert_eq!(a.source, "path+../a");
}

#[test]
fn the_same_graph_renders_the_same_bytes_from_any_checkout_location() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    graph(d1.path());
    graph(d2.path());
    let t1 = render(&plan(&d1.path().join("app"), &BTreeMap::new(), Policy::Refresh).unwrap());
    let t2 = render(&plan(&d2.path().join("app"), &BTreeMap::new(), Policy::Refresh).unwrap());
    assert_eq!(t1, t2, "no absolute path and no timestamp may reach the lock");
    assert!(!t1.contains(d1.path().to_string_lossy().as_ref()));
}

#[test]
fn a_changed_path_dependency_is_drift_and_update_clears_it() {
    let d = tempfile::tempdir().unwrap();
    graph(d.path());
    let app = d.path().join("app");
    update(&app).unwrap();
    assert!(enforce(&app, true).is_ok());
    write(d.path(), "b/src/lib.wj", "pub fn fb() -> int { 99 }\n");
    let strict = enforce(&app, true).unwrap_err();
    assert!(strict.contains("dependency `b`") && strict.contains("wj lock update"), "{strict}");
    let lenient = enforce(&app, false).unwrap();
    assert_eq!(lenient.len(), 1, "a plain build warns about a path dependency under development: {lenient:?}");
    update(&app).unwrap();
    assert!(enforce(&app, true).is_ok());
}

#[test]
fn a_new_or_removed_dependency_is_always_an_error() {
    let d = tempfile::tempdir().unwrap();
    graph(d.path());
    let app = d.path().join("app");
    update(&app).unwrap();
    write(d.path(), "d/wj.toml", "[package]\nname = \"d\"\n");
    write(d.path(), "d/src/lib.wj", "pub fn fd() -> int { 4 }\n");
    write(d.path(), "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nc = { path = \"../c\" }\na = { path = \"../a\" }\nd = { path = \"../d\" }\n");
    let e = enforce(&app, false).unwrap_err();
    assert!(e.contains("`d`") && e.contains("not in wj.lock"), "{e}");
    write(d.path(), "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nc = { path = \"../c\" }\n");
    let e = enforce(&app, false).unwrap_err();
    assert!(e.contains("lists `a`") || e.contains("lists `b`"), "{e}");
}

#[test]
fn a_cycle_is_named() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nx = { path = \"../x\" }\n");
    write(d.path(), "x/wj.toml", "[package]\nname = \"x\"\n\n[dependencies]\ny = { path = \"../y\" }\n");
    write(d.path(), "x/src/lib.wj", "pub fn f() -> int { 1 }\n");
    write(d.path(), "y/wj.toml", "[package]\nname = \"y\"\n\n[dependencies]\nx = { path = \"../x\" }\n");
    write(d.path(), "y/src/lib.wj", "pub fn g() -> int { 1 }\n");
    let e = plan(&d.path().join("app"), &BTreeMap::new(), Policy::Refresh).unwrap_err();
    assert!(e.contains("cycle") && e.contains("x") && e.contains("y"), "{e}");
}

#[test]
fn strict_without_a_lock_is_an_error_and_lenient_is_not() {
    let d = tempfile::tempdir().unwrap();
    graph(d.path());
    let app = d.path().join("app");
    assert!(enforce(&app, false).unwrap().is_empty());
    assert!(enforce(&app, true).unwrap_err().contains("needs a wj.lock"));
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git").current_dir(dir).args(["-c", "user.email=t@t", "-c", "user.name=t"]).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

#[test]
fn a_manifest_rev_that_disagrees_with_the_lock_fails_with_the_fix() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    write(r, "lib/src/lib.wj", "pub fn v() -> int { 1 }\n");
    git(&r.join("lib"), &["init", "-q"]);
    git(&r.join("lib"), &["add", "."]);
    git(&r.join("lib"), &["commit", "-qm", "one"]);
    let one = git(&r.join("lib"), &["rev-parse", "HEAD"]);
    write(r, "lib/src/lib.wj", "pub fn v() -> int { 2 }\n");
    git(&r.join("lib"), &["commit", "-qam", "two"]);
    let two = git(&r.join("lib"), &["rev-parse", "HEAD"]);
    let url = format!("file://{}", r.join("lib").display());
    let manifest = |rev: &str| format!("[package]\nname = \"app\"\n\n[dependencies]\nlib = {{ git = \"{url}\", rev = \"{rev}\" }}\n");
    write(r, "app/wj.toml", &manifest(&one));
    let app = r.join("app");
    update(&app).unwrap();
    let text = std::fs::read_to_string(app.join(LOCKFILE)).unwrap();
    assert!(text.contains(&one) && text.contains("hash = \"git-tree:"), "{text}");
    assert!(enforce(&app, true).is_ok());
    // the manifest moves to another revision; the lock is not updated
    write(r, "app/wj.toml", &manifest(&two));
    let e = enforce(&app, false).unwrap_err();
    assert!(e.contains(&two) && e.contains(&one) && e.contains("wj lock update"), "{e}");
    assert!(windjammer::package::graph::resolve(&app).unwrap_err().contains("wj lock update"), "resolve (interpreter, host builds) refuses too");
    // updating the lock accepts it
    update(&app).unwrap();
    assert!(std::fs::read_to_string(app.join(LOCKFILE)).unwrap().contains(&two));
    assert!(enforce(&app, true).is_ok());
}
