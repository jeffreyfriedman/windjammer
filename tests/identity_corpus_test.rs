//! Identity corpus (convergence plan C3): programs where two definitions share a name across modules,
//! types or namespaces. Every program must print its `// expect:` lines (computed by hand, independent of
//! every Windjammer backend) both in the interpreter and compiled (`wj build --no-cargo` + `rustc`).
//! A name collision that binds a call, a body or a variant literal to the wrong definition fails here.
//!
//! Standalone: `cargo test --release --test identity_corpus`.
//!
//! consumes: tests/identity_corpus/*.wj

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn programs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(root().join("tests/identity_corpus"))
        .expect("tests/identity_corpus")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "wj"))
        .collect();
    out.sort();
    out
}

fn expected(src: &str) -> String {
    src.lines()
        .filter_map(|l| l.trim().strip_prefix("// expect:"))
        .map(|l| format!("{}\n", l.trim()))
        .collect()
}

fn interpreted(file: &Path) -> Result<String, String> {
    let out = Command::new(wj()).arg("run").arg(file).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn compiled(file: &Path, work: &Path) -> Result<String, String> {
    let build = Command::new(wj())
        .args(["build", "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(work)
        .arg(file)
        .current_dir(work)
        .output()
        .map_err(|e| e.to_string())?;
    if !build.status.success() {
        return Err(format!("wj build failed:\n{}", String::from_utf8_lossy(&build.stderr)));
    }
    let stem = file.file_stem().unwrap().to_string_lossy().into_owned();
    let rs = work.join(format!("{stem}.rs"));
    let exe = work.join("prog");
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&exe)
        .arg(&rs)
        .output()
        .map_err(|e| e.to_string())?;
    if !rustc.status.success() {
        let rust = std::fs::read_to_string(&rs).unwrap_or_default();
        return Err(format!("rustc rejected:\n{}\n--- emitted ---\n{rust}", String::from_utf8_lossy(&rustc.stderr)));
    }
    let run = Command::new(&exe).output().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&run.stdout).into_owned())
}

#[test]
fn identity_corpus_programs_print_their_expectations_interpreted_and_compiled() {
    let progs = programs();
    assert!(progs.len() >= 6, "identity corpus is missing programs");
    let mut failures = Vec::new();
    for p in &progs {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let want = expected(&std::fs::read_to_string(p).unwrap());
        assert!(!want.is_empty(), "{name}: no `// expect:` lines");
        // Work on a copy: the compiler writes `.wj.meta` next to its input.
        let work = tempfile::TempDir::new().unwrap();
        let copy = work.path().join(&name);
        std::fs::copy(p, &copy).unwrap();
        match interpreted(&copy) {
            Ok(got) if got == want => {}
            Ok(got) => failures.push(format!("{name} [interpreter]: expected {want:?}, got {got:?}")),
            Err(e) => failures.push(format!("{name} [interpreter]: {e}")),
        }
        match compiled(&copy, work.path()) {
            Ok(got) if got == want => {}
            Ok(got) => failures.push(format!("{name} [compiled]: expected {want:?}, got {got:?}")),
            Err(e) => failures.push(format!("{name} [compiled]: {e}")),
        }
    }
    assert!(failures.is_empty(), "{} failure(s):\n{}", failures.len(), failures.join("\n\n"));
}
