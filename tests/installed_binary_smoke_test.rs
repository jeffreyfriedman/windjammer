//! Class-level gate: an *installed* `wj` (just the binary, nothing else) must behave
//! identically in every working directory, and must never read compiler sources,
//! templates, std, stdlib_meta or the runtime crate from the cwd or the build checkout.
//!
//! How it works
//! - The binary under test is copied alone into a temp prefix; the environment is
//!   cleared (empty PATH, isolated HOME), so nothing but the binary exists.
//! - Every offline subcommand case runs from several unrelated cwds, including decoy
//!   dirs that look like a compiler checkout (stray `src/lib.rs`, `Cargo.toml`, `std/`,
//!   `stdlib_meta/`, `templates/`, `crates/windjammer-runtime/`). Normalized exit code,
//!   output and produced files must equal the baseline run from a clean temp dir.
//! - `mutant_cwd_sensitive_binary_is_detected` proves the comparator can fail: a wrapper
//!   that reintroduces the historical `src/lib.rs` cwd sensitivity must be flagged.
//! - `source_tree_has_no_runtime_checkout_reads` statically forbids runtime reads of
//!   `CARGO_MANIFEST_DIR` / cwd-relative asset dirs in `src/`.
//! - Compiled `wj test` (needs cargo + a long runtime build) is opt-in:
//!   `WJ_SMOKE_COMPILED=1 cargo test --test all installed_binary_smoke`.

#[path = "common/installed_sandbox.rs"]
mod installed_sandbox;

use installed_sandbox::{real_wj, write, Sandbox};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

const TEMPLATES: [&str; 4] = ["cli", "web", "lib", "wasm"];

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    output: String,
    files: Vec<(String, u64)>,
}

fn normalize(s: &str, subs: &[(String, &str)]) -> String {
    let mut out = s.to_string();
    for (from, to) in subs {
        out = out.replace(from.as_str(), to);
    }
    // Strip ANSI escapes and wall-clock timings.
    let mut clean = String::new();
    let mut chars = out.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            clean.push(c);
        }
    }
    clean
        .lines()
        .filter(|l| {
            !l.contains("finished in") && !l.contains("Finished") && !l.contains("Compiling")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tree(dir: &Path) -> Vec<(String, u64)> {
    tree_normalized(dir, &[])
}

/// File list with content hashes; `subs` rewrites run-specific paths before hashing.
fn tree_normalized(dir: &Path, subs: &[(String, &str)]) -> Vec<(String, u64)> {
    fn walk(base: &Path, d: &Path, subs: &[(String, &str)], out: &mut Vec<(String, u64)>) {
        let Ok(rd) = fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
            if rel.starts_with(".git/") || rel == ".git" {
                continue;
            }
            if p.is_dir() {
                walk(base, &p, subs, out);
            } else {
                let bytes = fs::read(&p).unwrap_or_default();
                let bytes = match String::from_utf8(bytes) {
                    Ok(t) => subs
                        .iter()
                        .fold(t, |t, (f, to)| t.replace(f.as_str(), to))
                        .into_bytes(),
                    Err(e) => e.into_bytes(),
                };
                let h = bytes.iter().fold(1469598103934665603u64, |h, x| {
                    (h ^ *x as u64).wrapping_mul(1099511628211)
                });
                out.push((rel, h));
            }
        }
    }
    let mut v = Vec::new();
    walk(dir, dir, subs, &mut v);
    v.sort();
    v
}

/// One smoke case: `setup` populates a fresh scratch dir (`{w}`), `args` run in each cwd
/// (`{w}` = scratch dir, absolute), `stdin` is optional.
struct Case {
    name: &'static str,
    setup: fn(&Path, &Sandbox),
    args: Vec<String>,
    stdin: Option<&'static str>,
    /// Expected exit code; asserted on the baseline so a uniformly broken command fails too.
    expect_code: i32,
    expect_in_output: Option<&'static str>,
}

fn run_case(sb: &Sandbox, case: &Case, cwd_label: &str, cwd: &Path) -> Outcome {
    let scratch = TempDir::new().unwrap();
    let w = scratch.path().canonicalize().unwrap();
    (case.setup)(&w, sb);
    let args: Vec<String> = case
        .args
        .iter()
        .map(|a| a.replace("{w}", w.to_str().unwrap()))
        .collect();
    let mut cmd = sb.command(cwd);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("{}: spawn: {e}", case.name));
    {
        use std::io::Write;
        let mut si = child.stdin.take().unwrap();
        if let Some(s) = case.stdin {
            let _ = si.write_all(s.as_bytes());
        }
    }
    let out = child.wait_with_output().unwrap();
    let subs = vec![
        (w.to_string_lossy().to_string(), "<W>"),
        (cwd.to_string_lossy().to_string(), "<CWD>"),
        (sb.home.to_string_lossy().to_string(), "<HOME>"),
        (cwd_label.to_string(), "<LABEL>"),
    ];
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str("\n--stderr--\n");
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Outcome {
        code: out.status.code(),
        output: normalize(&text, &subs),
        files: tree_normalized(&w, &subs),
    }
}

/// Returns the list of divergences (empty = cwd-independent).
fn check_case(sb: &Sandbox, case: &Case) -> Vec<String> {
    let mut problems = Vec::new();
    let (base_label, base_cwd) = &sb.cwds[0];
    let base = run_case(sb, case, base_label, base_cwd);
    if base.code != Some(case.expect_code) {
        problems.push(format!(
            "{}: baseline exit {:?}, want {}\n{}",
            case.name, base.code, case.expect_code, base.output
        ));
    }
    if let Some(needle) = case.expect_in_output {
        if !base.output.contains(needle) {
            problems.push(format!(
                "{}: baseline output lacks {needle:?}\n{}",
                case.name, base.output
            ));
        }
    }
    for (label, cwd) in &sb.cwds[1..] {
        let got = run_case(sb, case, label, cwd);
        if got != base {
            problems.push(format!(
                "{}: behavior differs in cwd '{label}' vs '{base_label}'\n  code {:?} vs {:?}\n  files equal: {}\n--- baseline ---\n{}\n--- {label} ---\n{}",
                case.name, base.code, got.code, base.files == got.files, base.output, got.output
            ));
        }
    }
    problems
}

/// Fixture creation uses the real binary directly (not the binary/mutant under test).
fn new_project(w: &Path, _sb: &Sandbox, template: &str) {
    let st = Command::new(real_wj())
        .current_dir(w)
        .env_clear()
        .env("HOME", w)
        .args(["new", "proj", "--template", template])
        .output()
        .unwrap();
    assert!(
        st.status.success(),
        "wj new {template}: {}",
        String::from_utf8_lossy(&st.stderr)
    );
}

fn setup_cli(w: &Path, sb: &Sandbox) {
    new_project(w, sb, "cli")
}
fn setup_lib(w: &Path, sb: &Sandbox) {
    new_project(w, sb, "lib")
}
fn setup_web(w: &Path, sb: &Sandbox) {
    new_project(w, sb, "web")
}
fn setup_wasm(w: &Path, sb: &Sandbox) {
    new_project(w, sb, "wasm")
}
fn setup_none(_: &Path, _: &Sandbox) {}
fn setup_messy(w: &Path, _: &Sandbox) {
    write(
        &w.join("m.wj"),
        "fn main() {\n    let v = vec![1, 2]\n    println!(\"{}\", v.len())\n}\n",
    );
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    for t in TEMPLATES {
        let (setup, main): (fn(&Path, &Sandbox), &str) = match t {
            "cli" => (setup_cli, "main.wj"),
            "web" => (setup_web, "main.wj"),
            "lib" => (setup_lib, "lib.wj"),
            _ => (setup_wasm, "main.wj"),
        };
        let name: &'static str = Box::leak(format!("template {t}").into_boxed_str());
        let src = format!("{{w}}/proj/src/{main}");
        v.push(Case {
            name: Box::leak(format!("check {name}").into_boxed_str()),
            setup,
            args: vec!["check".into(), src.clone()],
            stdin: None,
            expect_code: 0,
            expect_in_output: Some("No errors found"),
        });
        v.push(Case {
            name: Box::leak(format!("build --no-cargo {name}").into_boxed_str()),
            setup,
            args: vec![
                "build".into(),
                "--no-cargo".into(),
                src.clone(),
                "-o".into(),
                "{w}/out".into(),
            ],
            stdin: None,
            expect_code: 0,
            expect_in_output: Some("Transpilation complete"),
        });
        v.push(Case {
            name: Box::leak(format!("lint {name}").into_boxed_str()),
            setup,
            args: vec!["lint".into(), src.clone()],
            stdin: None,
            expect_code: 0,
            expect_in_output: None,
        });
        v.push(Case {
            name: Box::leak(format!("fmt --check {name}").into_boxed_str()),
            setup,
            args: vec!["fmt".into(), "--check".into(), src.clone()],
            stdin: None,
            expect_code: 0,
            expect_in_output: None,
        });
        v.push(Case {
            name: Box::leak(format!("fix --check {name}").into_boxed_str()),
            setup,
            args: vec!["fix".into(), "--check".into(), src.clone()],
            stdin: None,
            expect_code: 0,
            expect_in_output: None,
        });
    }
    v.push(Case {
        name: "run --interpret cli",
        setup: setup_cli,
        args: s(&["run", "--interpret", "{w}/proj/src/main.wj", "World"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: Some("Hello, World!"),
    });
    v.push(Case {
        name: "test --interpret lib",
        setup: setup_lib,
        args: s(&["test", "--interpret", "{w}/proj/src/lib.wj"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: Some("4 passed"),
    });
    v.push(Case {
        name: "fix (rewrite)",
        setup: setup_messy,
        args: s(&["fix", "{w}/m.wj"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: None,
    });
    v.push(Case {
        name: "fmt (rewrite)",
        setup: setup_messy,
        args: s(&["fmt", "{w}/m.wj"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: None,
    });
    v.push(Case {
        name: "repl",
        setup: setup_none,
        args: s(&["repl"]),
        stdin: Some("1 + 2\n"),
        expect_code: 0,
        expect_in_output: Some("3"),
    });
    v.push(Case {
        name: "explain",
        setup: setup_none,
        args: s(&["explain", "WJ0001"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: None,
    });
    v.push(Case {
        name: "docs",
        setup: setup_none,
        args: s(&["docs", "hashmap", "entry"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: None,
    });
    v.push(Case {
        name: "self-install",
        setup: setup_none,
        args: s(&["self-install"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: None,
    });
    v
}

fn report(problems: Vec<String>) {
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}

#[test]
fn offline_subcommands_are_cwd_independent() {
    let sb = Sandbox::new(&real_wj());
    let mut problems = Vec::new();
    for case in cases() {
        problems.extend(check_case(&sb, &case));
    }
    report(problems);
}

#[test]
fn new_is_cwd_independent_for_every_template() {
    let sb = Sandbox::new(&real_wj());
    for t in TEMPLATES {
        let mut trees = Vec::new();
        for (label, cwd) in &sb.cwds {
            let scratch = TempDir::new().unwrap();
            let w = scratch.path().canonicalize().unwrap();
            // `wj new` creates `./<name>`; the cwd under test must not matter, so run it
            // from the scratch dir, then also from each cwd into a unique name.
            let out = sb
                .command(&w)
                .args(["new", "p", "--template", t])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{t}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let base = tree(&w.join("p"));
            // From the unrelated cwd (a unique project name, then compare contents).
            let name = format!("wjsmoke_{t}_{}", label.replace('-', "_"));
            let target = cwd.join(&name);
            let out = sb
                .command(cwd)
                .args(["new", &name, "--template", t])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{t}@{label}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let got = tree(&target);
            let _ = fs::remove_dir_all(&target);
            trees.push((label.to_string(), got));
        }
        // File names and sizes differ only by project name substitution; compare names.
        let names = |t: &Vec<(String, u64)>| t.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();
        let first = names(&trees[0].1);
        assert!(
            first.iter().any(|p| p == "wj.toml"),
            "{t}: missing wj.toml in {first:?}"
        );
        assert!(
            first.iter().any(|p| p == ".gitignore"),
            "{t}: missing .gitignore in {first:?}"
        );
        for (label, tr) in &trees {
            assert_eq!(
                names(tr),
                first,
                "template {t} file set differs in cwd '{label}'"
            );
        }
    }
}

#[test]
fn new_templates_survive_full_toolchain_free_pipeline() {
    // Each template: new -> check -> build --no-cargo -> fmt --check, project-relative cwd.
    let sb = Sandbox::new(&real_wj());
    for t in TEMPLATES {
        let scratch = TempDir::new().unwrap();
        let w = scratch.path().canonicalize().unwrap();
        let run = |args: &[&str]| {
            let o = sb.command(&w.join("p")).args(args).output().unwrap();
            (
                o.status.success(),
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                ),
            )
        };
        let o = sb
            .command(&w)
            .args(["new", "p", "--template", t])
            .output()
            .unwrap();
        assert!(o.status.success());
        let main = if t == "lib" {
            "src/lib.wj"
        } else {
            "src/main.wj"
        };
        for args in [
            vec!["check", main],
            vec!["build", "--no-cargo", main, "-o", "out"],
            vec!["fmt", "--check", main],
            vec!["lint", main],
        ] {
            let (ok, text) = run(&args);
            assert!(ok, "template {t}: wj {args:?} failed:\n{text}");
        }
        if t == "cli" {
            let (ok, text) = run(&["run", "--interpret", main, "Smoke"]);
            assert!(
                ok && text.contains("Hello, Smoke!"),
                "cli --interpret:\n{text}"
            );
        }
        if t == "lib" {
            let (ok, text) = run(&["test", "--interpret", main]);
            assert!(
                ok && text.contains("passed"),
                "lib test --interpret:\n{text}"
            );
        }
    }
}

#[test]
fn wj_test_inside_compiler_checkout_does_not_use_src_as_library() {
    // Regression for docs/known-issues (removed): `wj test` from the compiler checkout
    // printed "Using pre-built library at <checkout>/src" and cargo failed.
    let sb = Sandbox::new(&real_wj());
    let scratch = TempDir::new().unwrap();
    let w = scratch.path().canonicalize().unwrap();
    new_project(&w, &sb, "lib");
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let o = sb
        .command(&checkout)
        .args(["test", "--list", w.join("proj/src").to_str().unwrap()])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!text.contains("Using pre-built library"), "{text}");
    assert!(o.status.success(), "{text}");
}

/// Opt-in: compiled `wj test` needs cargo and builds the runtime crate (minutes).
#[test]
fn compiled_wj_test_is_cwd_independent() {
    if std::env::var_os("WJ_SMOKE_COMPILED").is_none() {
        eprintln!("skipped: set WJ_SMOKE_COMPILED=1 (needs cargo + network/cache)");
        return;
    }
    let cargo_bin = Command::new("sh")
        .arg("-c")
        .arg("command -v cargo")
        .output()
        .unwrap();
    let cargo = String::from_utf8_lossy(&cargo_bin.stdout)
        .trim()
        .to_string();
    if cargo.is_empty() {
        eprintln!("skipped: no cargo on PATH");
        return;
    }
    let tool_dir = Path::new(&cargo).parent().unwrap().to_path_buf();
    let sb = Sandbox::new(&real_wj());
    let target = TempDir::new().unwrap();
    let mut results = Vec::new();
    for (label, cwd) in &sb.cwds {
        let scratch = TempDir::new().unwrap();
        let w = scratch.path().canonicalize().unwrap();
        new_project(&w, &sb, "lib");
        let o = sb
            .command(cwd)
            .env("PATH", format!("{}:/usr/bin:/bin", tool_dir.display()))
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .env("CARGO_TARGET_DIR", target.path())
            .args(["test", w.join("proj/src").to_str().unwrap()])
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(
            o.status.success(),
            "compiled wj test failed in '{label}':\n{text}"
        );
        assert!(!text.contains("single file packages"), "{label}: {text}");
        results.push(text.contains("4 passed") || text.contains("test result: ok"));
    }
    assert!(results.iter().all(|r| *r), "{results:?}");
}

#[test]
fn mutant_cwd_sensitive_binary_is_detected() {
    // Self-test of the gate: a wrapper that reintroduces the historical cwd sensitivity
    // (treat ./src/lib.rs as a pre-built library) must make the comparator fail.
    let scratch = TempDir::new().unwrap();
    let mutant = scratch.path().join("wj-mutant");
    fs::write(
        &mutant,
        format!(
            "#!/bin/sh\nif [ -f src/lib.rs ]; then echo \"Using pre-built library at $PWD/src\" >&2; exit 101; fi\nexec {} \"$@\"\n",
            real_wj().display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mutant, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let sb = Sandbox::new(&mutant);
    let case = Case {
        name: "check lib (mutant)",
        setup: setup_lib_via_real,
        args: s(&["check", "{w}/proj/src/lib.wj"]),
        stdin: None,
        expect_code: 0,
        expect_in_output: None,
    };
    let problems = check_case(&sb, &case);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("stray-src-lib") || p.contains("compiler-checkout")),
        "gate failed to detect a cwd-sensitive binary: {problems:?}"
    );
}

fn setup_lib_via_real(w: &Path, sb: &Sandbox) {
    new_project(w, sb, "lib");
}

#[test]
fn source_tree_has_no_runtime_checkout_reads() {
    // Static gate: runtime code must not locate assets via the build checkout or the cwd.
    // Compile-time uses (`include_str!`, `include_bytes!`) and #[cfg(test)] code are fine.
    let forbidden = [
        "env!(\"CARGO_MANIFEST_DIR\")",
        "Path::new(\"std\")",
        "PathBuf::from(\"std\")",
        "\"./std\"",
        "Path::new(\"stdlib_meta\")",
        "Path::new(\"templates\")",
        "Path::new(\"agent_index\")",
        "Path::new(\"crates/windjammer-runtime",
    ];
    // Reviewed exceptions (file suffix, reason). Keep this list tiny.
    let allow: [(&str, &str); 1] = [(
        "agent_index/mod.rs",
        "maintainer-only: `wj agent-index` regenerates repo artifacts from a checkout",
    )];
    let mut hits = Vec::new();
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        if name == "tests.rs" || name.ends_with("_tests.rs") || name.ends_with("_test.rs") {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        // Everything after the first `#[cfg(test)]` module marker is test code.
        let prod = text.split("\n#[cfg(test)]").next().unwrap();
        for (i, line) in prod.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") || t.contains("include_str!") || t.contains("include_bytes!") {
                continue;
            }
            for pat in forbidden {
                if line.contains(pat) && !allow.iter().any(|(suffix, _)| f.ends_with(suffix)) {
                    hits.push(format!("{}:{}: {}", f.display(), i + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "runtime reads relative to the checkout/cwd (use crate::embedded_assets):\n{}",
        hits.join("\n")
    );
}
