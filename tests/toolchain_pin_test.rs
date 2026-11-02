//! Gates: a project pin (`[toolchain] wj` in wj.toml, or `WJ_COMMIT` in the ancestor compiler-pin file) that the running
//! `wj` does not satisfy stops build/run/test/check with the exact install command; the override warns.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn wj(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_wj"));
    c.args(args).current_dir(dir).env("WJ_NO_STALE_WARN", "1").env_remove("WJ_ALLOW_TOOLCHAIN_MISMATCH");
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn identity() -> serde_json::Value {
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["version", "--json"]).output().unwrap();
    serde_json::from_slice(&o.stdout).unwrap()
}

fn project(pin: Option<&str>) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let toolchain = pin.map(|p| format!("\n[toolchain]\nwj = \"{p}\"\n")).unwrap_or_default();
    fs::write(d.path().join("wj.toml"), format!("[package]\nname = \"pinned\"\nversion = \"0.1.0\"\n{toolchain}")).unwrap();
    fs::write(d.path().join("main.wj"), "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    d
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// The four commands the pin guards, each invoked the way a user would.
fn guarded(d: &Path) -> Vec<(&'static str, Vec<String>)> {
    let f = d.join("main.wj").display().to_string();
    vec![
        ("check", vec!["check".into(), f.clone()]),
        ("run", vec!["run".into(), "--interpret".into(), f.clone()]),
        ("test", vec!["test".into(), d.display().to_string()]),
        ("build", vec!["build".into(), f, "--no-cargo".into(), "-o".into(), d.join("out").display().to_string()]),
    ]
}

#[test]
fn mismatching_pin_is_an_error_with_the_install_command_for_all_four_commands() {
    let d = project(Some("0000000"));
    for (name, args) in guarded(d.path()) {
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let o = wj(d.path(), &a, &[]);
        let err = stderr(&o);
        assert_eq!(o.status.code(), Some(1), "{name}: {err}");
        assert!(err.contains("pins wj `0000000`"), "{name}: {err}");
        assert!(err.contains("cargo install --git https://github.com/jeffreyfriedman/windjammer.git --rev 0000000 --bin wj --locked --force"), "{name}: {err}");
        assert!(err.contains("WJ_ALLOW_TOOLCHAIN_MISMATCH=1"), "{name}: {err}");
    }
}

#[test]
fn override_downgrades_the_error_to_a_warning_and_the_command_runs() {
    let d = project(Some("0000000"));
    let f = d.path().join("main.wj").display().to_string();
    let o = wj(d.path(), &["run", "--interpret", &f], &[("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1")]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "hi\n");
    assert!(stderr(&o).contains("toolchain mismatch allowed"), "{}", stderr(&o));
}

#[test]
fn matching_pins_pass_silently_by_commit_prefix_and_by_version() {
    let id = identity();
    let Some(commit) = id["commit"].as_str() else { return };
    for pin in [&commit[..7], &commit[..20], commit, id["version"].as_str().unwrap()] {
        let d = project(Some(pin));
        let f = d.path().join("main.wj").display().to_string();
        let o = wj(d.path(), &["run", "--interpret", &f], &[]);
        assert_eq!(o.status.code(), Some(0), "pin {pin}: {}", stderr(&o));
        assert!(!stderr(&o).contains("toolchain"), "pin {pin}: {}", stderr(&o));
    }
}

#[test]
fn ancestor_pin_file_is_honoured_and_wj_toml_overrides_it() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join(windjammer::toolchain::pin::PIN_FILE), "# pin\n\n```\nWJ_REPO=https://example.com/wj.git\nWJ_COMMIT=1111111111111111111111111111111111111111\n```\n").unwrap();
    let app = root.path().join("apps").join("one");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("main.wj"), "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    let o = wj(&app, &["check", "main.wj"], &[]);
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("--git https://example.com/wj.git --rev 1111111111111111111111111111111111111111"), "{err}");
    // the project's own wj.toml pin wins over the repo-level file
    let id = identity();
    let Some(commit) = id["commit"].as_str() else { return };
    fs::write(app.join("wj.toml"), format!("[package]\nname=\"one\"\n[toolchain]\nwj = \"{}\"\n", &commit[..9])).unwrap();
    let o = wj(&app, &["check", "main.wj"], &[]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

#[test]
fn unguarded_commands_ignore_the_pin() {
    let d = project(Some("0000000"));
    let o = wj(d.path(), &["version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    let o = wj(d.path(), &["doctor", "--json"], &[]);
    assert!(!stderr(&o).contains("pins wj"), "{}", stderr(&o));
}
