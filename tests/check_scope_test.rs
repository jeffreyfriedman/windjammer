//! Which files `wj check` reads (`check::scope`): stale generated or ignored directories must never inflate the
//! file set or change a diagnostic. Each test below is mutation-checked against the hunk it guards.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use windjammer::check::files;

/// The tests share the process's git configuration and temp area; run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (rel, text) in files {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

fn names(root: &Path, found: &[PathBuf]) -> Vec<String> {
    let root = root.canonicalize().unwrap();
    let mut v: Vec<String> = found
        .iter()
        .map(|f| f.canonicalize().unwrap().strip_prefix(&root).unwrap().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

const MANIFEST: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\n";
const OK: &str = "fn main() {\n    println(\"ok\")\n}\n";

fn git(dir: &Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

#[test]
fn generated_directories_are_not_sources() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[
        ("wj.toml", MANIFEST),
        ("src/a.wj", OK),
        ("gen/a.wj", OK),
        ("build/b.wj", OK),
        ("target/c.wj", OK),
        (".wj-cache/d.wj", OK),
        ("node_modules/e.wj", OK),
        ("out/f.wj", OK),
    ]);
    // `out` is output only when it holds a Cargo.toml or package.json
    let found = files::collect(d.path()).unwrap();
    assert_eq!(names(d.path(), &found), ["out/f.wj", "src/a.wj"]);
    std::fs::write(d.path().join("out/Cargo.toml"), "").unwrap();
    let (found, scope) = files::collect_scoped(d.path(), false).unwrap();
    assert_eq!(names(d.path(), &found), ["src/a.wj"]);
    assert_eq!(scope.skipped, 6);
}

#[test]
fn a_directory_of_stale_copies_is_neither_counted_nor_checked() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[("wj.toml", MANIFEST), ("src/main.wj", OK)]);
    for i in 0..60 {
        let p = d.path().join(format!("gen/copy_{i}.wj"));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "fn broken( {\n").unwrap();
    }
    let checked = windjammer::check::check_path(d.path(), &windjammer::check::CheckOptions { use_cache: false }).unwrap();
    assert_eq!(checked.report.summary.files_checked, 1);
    assert_eq!(checked.report.summary.errors, 0);
    assert_eq!(checked.scope.skipped, 60);
    assert_eq!(checked.scope.skipped_line().unwrap(), "skipped 60 ignored/generated files (see --no-ignore)");
}

#[test]
fn no_ignore_walks_everything() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[("wj.toml", MANIFEST), ("src/a.wj", OK), ("gen/a.wj", OK)]);
    let (found, scope) = files::collect_scoped(d.path(), true).unwrap();
    assert_eq!(names(d.path(), &found), ["gen/a.wj", "src/a.wj"]);
    assert!(scope.skipped_line().is_none());
}

#[test]
fn wj_toml_include_and_exclude_override_the_defaults() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let manifest = format!("{MANIFEST}\n[check]\nexclude = [\"legacy\", \"src/old/one.wj\"]\ninclude = [\"gen\"]\n");
    let d = project(&[
        ("wj.toml", &manifest),
        ("src/a.wj", OK),
        ("src/legacy/b.wj", OK),
        ("src/old/one.wj", OK),
        ("src/old/two.wj", OK),
        ("gen/keep.wj", OK),
        ("build/drop.wj", OK),
    ]);
    let found = files::collect(d.path()).unwrap();
    assert_eq!(names(d.path(), &found), ["gen/keep.wj", "src/a.wj", "src/old/two.wj"]);
}

#[test]
fn git_ignored_paths_are_left_out_and_a_plain_tree_falls_back_to_the_defaults() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[
        ("wj.toml", MANIFEST),
        (".gitignore", "vendored_copy/\n*_gen.wj\n"),
        ("src/a.wj", OK),
        ("src/a_gen.wj", OK),
        ("vendored_copy/a.wj", OK),
        ("vendored_copy/deep/b.wj", OK),
    ]);
    // not a work tree yet: only the default list applies
    let found = files::collect(d.path()).unwrap();
    assert_eq!(found.len(), 4);
    git(d.path(), &["init", "-q"]);
    let (found, scope) = files::collect_scoped(d.path(), false).unwrap();
    assert_eq!(names(d.path(), &found), ["src/a.wj"]);
    assert_eq!(scope.skipped, 3);
    // a tracked file is never ignored, whatever the patterns say
    git(d.path(), &["add", "-f", "src/a_gen.wj"]);
    let found = files::collect(d.path()).unwrap();
    assert_eq!(names(d.path(), &found), ["src/a.wj", "src/a_gen.wj"]);
}

#[test]
fn an_ignored_directory_named_on_the_command_line_is_still_checked() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[(".gitignore", "vendored_copy/\n"), ("vendored_copy/a.wj", OK)]);
    git(d.path(), &["init", "-q"]);
    let found = files::collect(&d.path().join("vendored_copy")).unwrap();
    assert_eq!(found.len(), 1);
}

#[test]
fn a_stale_generated_directory_gets_a_warning_only_when_older_than_the_sources() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[("wj.toml", MANIFEST), ("src/a.wj", OK), ("gen/lib.rs", "// old\n")]);
    let set = |rel: &str, secs_ago: u64| {
        let f = std::fs::OpenOptions::new().write(true).open(d.path().join(rel)).unwrap();
        f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago)).unwrap();
    };
    set("gen/lib.rs", 1000);
    set("src/a.wj", 10);
    let (_, scope) = files::collect_scoped(d.path(), false).unwrap();
    assert_eq!(
        scope.stale_generated_messages(),
        ["generated directory gen/ is older than its sources (stale output is ignored by check; run wj build or wj clean --generated)"]
    );
    set("gen/lib.rs", 1);
    let (_, scope) = files::collect_scoped(d.path(), false).unwrap();
    assert!(scope.stale_generated_messages().is_empty());
}

/// The widths gate predicts from Rust it generates itself: the build's scratch output must hold nothing copied back
/// from the project's own `gen/` or `build/`, whose stale text would be read in place of the fresh Rust.
#[test]
fn the_build_the_widths_gate_reads_never_contains_a_stale_generated_directory() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let stale = "pub fn f() { let a: u64 = 1; let b: i64 = 2; let _ = a < b; }\n";
    let d = project(&[
        ("wj.toml", MANIFEST),
        ("lib.wj", "pub mod x;\n"),
        ("x.wj", "pub fn f() -> int {\n    1\n}\n"),
        ("gen/mod.rs", "pub mod x;\n"),
        ("gen/x.rs", stale),
        ("build/mod.rs", "pub mod x;\n"),
        ("build/x.rs", stale),
    ]);
    let scratch = tempfile::tempdir().unwrap();
    let built = windjammer::cli::widths_gate::built_in_process(d.path(), scratch.path()).expect("the project builds");
    assert!(!built.is_empty());
    for (p, text) in &built {
        let rel = p.strip_prefix(scratch.path()).unwrap();
        assert!(!rel.starts_with("gen") && !rel.starts_with("build"), "stale output copied into the build: {}", rel.display());
        assert!(!text.contains("let a: u64"), "stale text in {}", rel.display());
    }
    assert!(windjammer::cli::widths_gate::predicted_widths(d.path()).is_empty());
}

#[test]
fn clean_generated_removes_untracked_output_and_keeps_tracked_directories() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let d = project(&[("wj.toml", MANIFEST), ("src/a.wj", OK), ("gen/x.rs", "// out\n"), ("build/kept.rs", "// hand-written\n")]);
    git(d.path(), &["init", "-q"]);
    git(d.path(), &["add", "wj.toml", "src/a.wj", "build/kept.rs"]);
    git(d.path(), &["commit", "-q", "-m", "init"]);
    windjammer::cli::clean_generated::run(d.path(), false).unwrap();
    assert!(d.path().join("gen/x.rs").is_file(), "a dry run removes nothing");
    windjammer::cli::clean_generated::run(d.path(), true).unwrap();
    assert!(!d.path().join("gen").exists());
    assert!(d.path().join("build/kept.rs").is_file(), "tracked files are never removed");
    assert!(d.path().join("src/a.wj").is_file());
}
