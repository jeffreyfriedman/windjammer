//! `wj eject` gate.
//!
//! - `ejected_rust_is_byte_identical_to_wj_build`: for every `// rust: ok` verified example and
//!   every `wj new` template, the ejected `.rs` files equal `wj build --no-cargo` output byte for
//!   byte (so behavior is identical by construction), and the ejected tree is self-contained: every
//!   `path` dependency is relative and resolves inside the output, and no file outside `vendor/`
//!   mentions the build checkout or any absolute temp path.
//! - `eject_refuses_non_empty_output`, `multi_file_project_ejects_like_wj_build`.
//! - Opt-in (`WJ_SMOKE_COMPILED=1`, needs cargo): `ejected_projects_build_and_run_anywhere` moves
//!   every ejected project to a fresh directory and requires `cargo run` / `cargo test` there to
//!   match the `wj build` output in place (build success, exit code, stdout). Mismatches against the
//!   examples' `// expect:` oracle that both share are reported as compiled-path gaps.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use windjammer::agent_index::verified::{load_examples, RustStatus};

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const TEMPLATES: [&str; 4] = ["cli", "web", "lib", "wasm"];

fn verified_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/verified")
}

fn wj(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(WJ)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn ok(o: &std::process::Output, what: &str) {
    assert!(
        o.status.success(),
        "{what} failed:\n{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}

/// `(display name, source path relative to its scratch dir)` with sources materialized in `scratch`.
fn corpus(scratch: &Path) -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    for (h, _) in load_examples(&verified_dir()).unwrap() {
        if h.rust != RustStatus::Ok {
            continue;
        }
        let d = scratch.join(format!("ex_{}", h.name));
        fs::create_dir_all(&d).unwrap();
        let f = d.join(format!("{}.wj", h.name));
        fs::copy(verified_dir().join(format!("{}.wj", h.name)), &f).unwrap();
        v.push((h.name.clone(), f));
    }
    for t in TEMPLATES {
        ok(
            &wj(scratch, &["new", &format!("tpl_{t}"), "--template", t]),
            "wj new",
        );
        let main = if t == "lib" { "lib.wj" } else { "main.wj" };
        v.push((
            format!("template {t}"),
            scratch.join(format!("tpl_{t}/src/{main}")),
        ));
    }
    v
}

fn rs_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "rs") {
            out.push((
                p.file_name().unwrap().to_string_lossy().to_string(),
                fs::read(&p).unwrap(),
            ));
        }
    }
    out.sort();
    out
}

fn self_contained_problems(out: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(out.join("Cargo.toml")).unwrap()).unwrap();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(deps) = manifest.get(section).and_then(|d| d.as_table()) else {
            continue;
        };
        for (name, spec) in deps {
            if let Some(p) = spec.get("path").and_then(|p| p.as_str()) {
                let resolved = out.join(p);
                if Path::new(p).is_absolute()
                    || !resolved.join("Cargo.toml").is_file()
                    || !resolved
                        .canonicalize()
                        .unwrap()
                        .starts_with(out.canonicalize().unwrap())
                {
                    problems.push(format!(
                        "dependency {name} path {p:?} is not inside the output"
                    ));
                }
            }
        }
    }
    let checkout = env!("CARGO_MANIFEST_DIR");
    for e in fs::read_dir(out).unwrap().flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let text = fs::read_to_string(&p).unwrap_or_default();
        let is_manifest = p.file_name().is_some_and(|n| n == "Cargo.toml");
        let mut needles = vec![checkout, "wj-embedded"];
        if is_manifest {
            needles.extend(["/private/", "/var/folders/", "/tmp/", "/Users/", "/home/"]);
        }
        for needle in needles {
            if text.contains(needle) {
                problems.push(format!("{} mentions {needle:?}", p.display()));
            }
        }
    }
    problems
}

#[test]
fn ejected_rust_is_byte_identical_to_wj_build() {
    let scratch = TempDir::new().unwrap();
    let s = scratch.path().canonicalize().unwrap();
    let items = corpus(&s);
    assert!(items.len() >= 50, "corpus too small: {}", items.len());
    let mut failures = Vec::new();
    let mut nondeterministic = Vec::new();
    for (name, src) in &items {
        let tag = name.replace(' ', "_");
        let built = s.join(format!("built_{tag}"));
        let ejected = s.join(format!("ejected_{tag}"));
        let b = wj(
            &s,
            &[
                "build",
                "--no-cargo",
                src.to_str().unwrap(),
                "-o",
                built.to_str().unwrap(),
            ],
        );
        let e = wj(
            &s,
            &[
                "eject",
                src.to_str().unwrap(),
                "-o",
                ejected.to_str().unwrap(),
            ],
        );
        if !b.status.success() || !e.status.success() {
            failures.push(format!(
                "{name}: build ok={} eject ok={}\n{}",
                b.status.success(),
                e.status.success(),
                String::from_utf8_lossy(&e.stderr)
            ));
            continue;
        }
        let re = rs_files(&ejected);
        let mut variants = vec![rs_files(&built)];
        // `wj build` codegen is not fully deterministic for a few programs (compiler bug, see
        // `nondeterministic` below): the ejected sources must equal *some* `wj build` output.
        while !variants.contains(&re) && variants.len() < 8 {
            let again = s.join(format!("rebuilt_{tag}_{}", variants.len()));
            let _ = fs::remove_dir_all(src.parent().unwrap().join(".wj-cache"));
            let o = wj(
                &s,
                &[
                    "build",
                    "--no-cargo",
                    src.to_str().unwrap(),
                    "-o",
                    again.to_str().unwrap(),
                ],
            );
            ok(&o, "wj build (repeat)");
            variants.push(rs_files(&again));
        }
        let distinct: std::collections::BTreeSet<_> = variants.iter().collect();
        if distinct.len() > 1 {
            nondeterministic.push(name.clone());
        }
        if variants[0].is_empty() || !variants.contains(&re) {
            failures.push(format!(
                "{name}: ejected .rs differ from every wj build output ({:?} vs {:?})",
                variants[0].iter().map(|f| &f.0).collect::<Vec<_>>(),
                re.iter().map(|f| &f.0).collect::<Vec<_>>()
            ));
        }
        for p in self_contained_problems(&ejected) {
            failures.push(format!("{name}: {p}"));
        }
    }
    if !nondeterministic.is_empty() {
        eprintln!("wj build codegen varied between runs for: {nondeterministic:?}");
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn multi_file_project_ejects_like_wj_build() {
    let scratch = TempDir::new().unwrap();
    let s = scratch.path().canonicalize().unwrap();
    let src = s.join("proj/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        s.join("proj/wj.toml"),
        "[package]\nname = \"proj\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        src.join("main.wj"),
        "use crate::util\n\nfn main() {\n    println(\"${util::answer()}\")\n}\n",
    )
    .unwrap();
    fs::write(src.join("util.wj"), "pub fn answer() -> int {\n    42\n}\n").unwrap();
    let p = src.to_str().unwrap();
    ok(
        &wj(&s, &["build", "--no-cargo", p, "-o", "built"]),
        "wj build",
    );
    ok(&wj(&s, &["eject", p, "-o", "ejected"]), "wj eject");
    let (b, e) = (rs_files(&s.join("built")), rs_files(&s.join("ejected")));
    assert!(
        b.len() >= 2,
        "expected main.rs + util.rs, got {:?}",
        b.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    assert_eq!(b, e);
    assert!(self_contained_problems(&s.join("ejected")).is_empty());
}

#[test]
fn eject_refuses_non_empty_output() {
    let scratch = TempDir::new().unwrap();
    let s = scratch.path();
    fs::write(s.join("a.wj"), "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    fs::create_dir_all(s.join("out")).unwrap();
    fs::write(s.join("out/keep.txt"), "user data").unwrap();
    let o = wj(s, &["eject", "a.wj", "-o", "out"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("not empty"));
    assert_eq!(
        fs::read_to_string(s.join("out/keep.txt")).unwrap(),
        "user data"
    );
}

/// `(build succeeded, exit code, stdout)` of `cargo <args>` in `dir`.
fn cargo_outcome(dir: &Path, target: &Path, args: &[&str]) -> (bool, Option<i32>, String, String) {
    let build = Command::new("cargo")
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target)
        .args(["build", "-q"])
        .output()
        .unwrap();
    if !build.status.success() {
        let err = String::from_utf8_lossy(&build.stderr)
            .chars()
            .take(600)
            .collect();
        return (false, None, String::new(), err);
    }
    let o = Command::new("cargo")
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target)
        .args(args)
        .output()
        .unwrap();
    (
        true,
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::new(),
    )
}

#[test]
fn ejected_projects_build_and_run_anywhere() {
    if std::env::var_os("WJ_SMOKE_COMPILED").is_none() {
        eprintln!("skipped: set WJ_SMOKE_COMPILED=1 (needs cargo; builds every ejected project)");
        return;
    }
    let scratch = TempDir::new().unwrap();
    let s = scratch.path().canonicalize().unwrap();
    let elsewhere = TempDir::new().unwrap();
    let target = std::env::var_os("WJ_EJECT_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| s.join("target"));
    let headers: std::collections::HashMap<String, _> = load_examples(&verified_dir())
        .unwrap()
        .into_iter()
        .map(|(h, _)| (h.name.clone(), h))
        .collect();
    let mut failures = Vec::new();
    let mut compiler_gaps = Vec::new();
    let mut matched = 0;
    for (name, src) in corpus(&s) {
        let tag = name.replace(' ', "_");
        let built = s.join(format!("built_{tag}"));
        let ejected = s.join(format!("ejected_{tag}"));
        ok(
            &wj(
                &s,
                &[
                    "build",
                    "--no-cargo",
                    src.to_str().unwrap(),
                    "-o",
                    built.to_str().unwrap(),
                ],
            ),
            "build",
        );
        ok(
            &wj(
                &s,
                &[
                    "eject",
                    src.to_str().unwrap(),
                    "-o",
                    ejected.to_str().unwrap(),
                ],
            ),
            "eject",
        );
        // Move the ejected project: it must not depend on where it was generated.
        let moved = elsewhere.path().join(&tag);
        fs::rename(&ejected, &moved).unwrap();
        let args: Vec<&str> = match name.as_str() {
            "template lib" => vec!["test", "-q"],
            "template web" => vec!["build", "-q"], // network-dependent output; building is the contract
            n if n.starts_with("template ") => vec!["run", "-q", "--", "World"],
            _ => vec!["run", "-q"],
        };
        let b = cargo_outcome(&built, &target, &args);
        let e = cargo_outcome(&moved, &target, &args);
        if (b.0, b.1, &b.2) != (e.0, e.1, &e.2) {
            failures.push(format!(
                "{name}: wj build output vs ejected differ\n  built: ok={} exit={:?} {:?} {}\n  ejected: ok={} exit={:?} {:?} {}",
                b.0, b.1, b.2, b.3, e.0, e.1, e.2, e.3
            ));
            continue;
        }
        matched += 1;
        if let Some(h) = headers.get(&name) {
            let got: Vec<String> = e.2.lines().map(str::to_string).collect();
            if !e.0 || got != h.expect {
                compiler_gaps.push(format!(
                    "{name}: compiled output {:?} != expect {:?} {}",
                    got, h.expect, e.3
                ));
            }
        }
    }
    eprintln!(
        "eject: {matched} projects behave identically to wj build; {} compiled-path gaps shared by both:\n{}",
        compiler_gaps.len(),
        compiler_gaps.join("\n")
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
