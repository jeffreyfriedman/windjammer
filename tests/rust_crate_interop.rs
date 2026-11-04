//! ADR-0026: Rust crate dependencies declared in wj.toml, used from .wj through a signature surface
//! (`rust/<crate>.wji`). Run: `cargo test --release --test rust_crate_interop`.
//!
//! Oracles are independent of the compiler: Cargo.toml text, rustc/cargo accepting the generated crate and the
//! real `strsim` crate's output (levenshtein("kitten", "sitting") == 3 is the textbook value).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SURFACE: &str = "\
/// Edit distance between two strings.
@pure
pub fn levenshtein(a: string, b: string) -> usize

@pure
pub fn jaro(a: string, b: string) -> f64

pub fn osa_distance(a: string, b: string) -> usize
";

struct Project {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new(manifest_extra: &str, main: &str, surface: Option<&str>) -> Self {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("app");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("wj.toml"),
            format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nstrsim = \"0.11\"\n{manifest_extra}"),
        )
        .unwrap();
        std::fs::write(root.join("src/main.wj"), main).unwrap();
        if let Some(s) = surface {
            std::fs::create_dir_all(root.join("rust")).unwrap();
            std::fs::write(root.join("rust/strsim.wji"), s).unwrap();
        }
        Project { _tmp: tmp, root }
    }

    fn wj(&self, args: &[&str]) -> Output {
        Command::new(WJ).args(args).current_dir(&self.root).env("NO_COLOR", "1").output().unwrap()
    }

    fn transpile(&self) -> (Output, PathBuf) {
        let out = self.root.join("out");
        let o = self.wj(&["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
        (o, out)
    }

    fn check(&self) -> (bool, String) {
        let o = self.wj(&["check", "."]);
        (o.status.success(), text(&o))
    }
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

const MAIN: &str = "\
use strsim

fn main() {
    let a = \"kitten\"
    let b = \"sitting\"
    let d = strsim::levenshtein(a, b)
    println(\"distance ${d}\")
    let j = strsim::jaro(\"martha\", \"marhta\")
    println(\"jaro ${j}\")
}
";

#[test]
fn rust_dependency_lines_reach_generated_cargo_toml() {
    let p = Project::new("hex = { version = \"0.4\", features = [\"serde\"] }\n", MAIN, Some(SURFACE));
    let (o, out) = p.transpile();
    assert!(o.status.success(), "{}", text(&o));
    let cargo = read(&out.join("Cargo.toml"));
    assert!(cargo.contains("strsim = \"0.11\""), "{cargo}");
    assert!(cargo.contains("hex = { version = \"0.4\", features = [\"serde\"] }"), "{cargo}");
}

#[test]
fn surface_gives_codegen_the_boundary_signature() {
    let p = Project::new("", MAIN, Some(SURFACE));
    let (o, out) = p.transpile();
    assert!(o.status.success(), "{}", text(&o));
    let rs = read(&out.join("main.rs"));
    assert!(!rs.contains("compile_error!"), "{rs}");
    // `string` parameters of a surface are passed borrowed (Rust `&str`)
    assert!(rs.contains("strsim::levenshtein(&a, &b)"), "{rs}");
    assert!(rs.contains("strsim::jaro(\"martha\", \"marhta\")"), "{rs}");
}

#[test]
fn without_a_surface_the_build_still_fails_closed() {
    // mutation guard: the surface is what removes the boundary error, not something else in this test setup.
    // `wj build` generates a surface from the crate source when it finds one; an empty Cargo home leaves none.
    let p = Project::new("", MAIN, None);
    let empty_home = tempfile::TempDir::new().unwrap();
    let out = p.root.join("out");
    let o = Command::new(WJ)
        .args(["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"])
        .current_dir(&p.root)
        .env("NO_COLOR", "1")
        .env("CARGO_HOME", empty_home.path())
        .output()
        .unwrap();
    assert!(text(&o).contains("no signature surface for Rust crate `strsim`") && text(&o).contains("`wj rust-surface strsim`"), "{}", text(&o));
    let rs = std::fs::read_to_string(out.join("main.rs")).unwrap_or_default();
    assert!(!o.status.success() || rs.contains("missing boundary signature"), "{}\n{rs}", text(&o));
}

#[test]
fn surface_is_not_compiled_as_a_module() {
    let p = Project::new("", MAIN, Some(SURFACE));
    let (o, out) = p.transpile();
    assert!(o.status.success(), "{}", text(&o));
    assert!(!out.join("strsim.rs").exists() && !out.join("rust").exists());
    assert!(!read(&out.join("main.rs")).contains("mod strsim"));
}

#[test]
fn check_accepts_calls_that_match_the_surface() {
    let p = Project::new("", MAIN, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(ok, "{out}");
    assert!(!out.contains("WJ0050"), "{out}");
}

#[test]
fn check_rejects_a_function_the_surface_does_not_declare() {
    let main = MAIN.replace("strsim::levenshtein(a, b)", "strsim::levenstein(a, b)");
    let p = Project::new("", &main, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(!ok, "{out}");
    assert!(out.contains("error[WJ0050]"), "{out}");
    assert!(out.contains("`strsim::levenstein` is not declared in the signature surface of Rust crate `strsim`"), "{out}");
    assert!(out.contains("levenshtein"), "suggestion expected: {out}");
    assert!(out.contains("main.wj:6"), "location expected: {out}");
}

#[test]
fn check_rejects_wrong_argument_count() {
    let main = MAIN.replace("strsim::levenshtein(a, b)", "strsim::levenshtein(a)");
    let p = Project::new("", &main, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(!ok, "{out}");
    assert!(out.contains("error[WJ0013]") && out.contains("`strsim::levenshtein` takes 2 arguments but 1 was supplied"), "{out}");
}

#[test]
fn check_rejects_a_literal_of_the_wrong_type() {
    let main = MAIN.replace("strsim::jaro(\"martha\", \"marhta\")", "strsim::jaro(\"martha\", 7)");
    let p = Project::new("", &main, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(!ok, "{out}");
    assert!(out.contains("error[WJ0003]") && out.contains("strsim::jaro"), "{out}");
}

#[test]
fn check_follows_a_direct_import() {
    let main = "use strsim::levenshtein\n\nfn main() {\n    println(\"${levenshtein(\"a\")}\")\n}\n";
    let p = Project::new("", main, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(!ok && out.contains("error[WJ0013]"), "{out}");
}

#[test]
fn check_rejects_a_surface_function_with_a_body() {
    let surface = "pub fn levenshtein(a: string, b: string) -> usize {\n    0\n}\n";
    let p = Project::new("", "use strsim\nfn main() {\n    println(\"${strsim::levenshtein(\"a\", \"b\")}\")\n}\n", Some(surface));
    let (ok, out) = p.check();
    assert!(!ok, "{out}");
    assert!(out.contains("error[WJ0050]") && out.contains("strsim.wji") && out.contains("has a body"), "{out}");
}

#[test]
fn build_stops_before_codegen_on_a_surface_error() {
    let main = MAIN.replace("strsim::levenshtein(a, b)", "strsim::levenstein(a, b)");
    let p = Project::new("", &main, Some(SURFACE));
    let (o, out) = p.transpile();
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("WJ0050"), "{}", text(&o));
    assert!(!out.join("main.rs").exists());
}

// ---- effects (ADR-0021 §11 applied to crates) ----

const CAPS: &str = "\n[capabilities]\n";

#[test]
fn pure_surface_functions_need_no_grant() {
    let main = "use strsim\nfn main() {\n    println(\"${strsim::levenshtein(\"a\", \"b\")}\")\n}\n";
    let p = Project::new(CAPS, main, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(ok, "{out}");
}

#[test]
fn undeclared_surface_function_is_ffi_of_the_crate() {
    let main = "use strsim\nfn main() {\n    println(\"${strsim::osa_distance(\"a\", \"b\")}\")\n}\n";
    let p = Project::new(CAPS, main, Some(SURFACE));
    let (ok, out) = p.check();
    assert!(!ok, "{out}");
    assert!(out.contains("error[WJ0044]") && out.contains("ffi = [\"strsim\"]"), "{out}");
    let granted = Project::new("\n[capabilities]\nffi = [\"strsim\"]\n", main, Some(SURFACE));
    let (ok, out) = granted.check();
    assert!(ok, "{out}");
}

#[test]
fn crate_without_surface_is_ffi_of_the_crate() {
    let main = "use strsim\nfn main() {\n    println(\"${strsim::levenshtein(\"a\", \"b\")}\")\n}\n";
    let p = Project::new(CAPS, main, None);
    let (ok, out) = p.check();
    assert!(!ok, "{out}");
    assert!(out.contains("error[WJ0044]") && out.contains("ffi = [\"strsim\"]"), "{out}");
}

#[test]
fn explain_effects_lists_the_crate() {
    let main = "use strsim\nfn main() {\n    println(\"${strsim::osa_distance(\"a\", \"b\")}\")\n}\n";
    let p = Project::new("", main, Some(SURFACE));
    let o = p.wj(&["explain-effects", "."]);
    assert!(text(&o).contains("ffi") && text(&o).contains("strsim"), "{}", text(&o));
}

// ---- interpreter ----

#[test]
fn interpreter_runs_surfaced_crate_calls() {
    // ADR-0031: the crate host serves surfaced functions (was: preflight `deps.rust_crate`, exit 2)
    let p = Project::new("", MAIN, Some(SURFACE));
    let o = p.wj(&["run", "--interpret"]);
    assert!(o.status.success(), "{}", text(&o));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("distance 3") && out.contains("jaro 0.944"), "{}", text(&o));
}

#[test]
fn interpreter_names_a_crate_call_it_cannot_carry_and_exits_2() {
    // a surface type the crate host cannot carry keeps the preflight report, naming the call
    let surface = "pub fn levenshtein(a: string, b: string) -> usize\npub fn tally(m: HashMap<string, int>) -> usize\n";
    let main = "use strsim\nuse std::collections::HashMap\n\nfn main() {\n    let m: HashMap<string, int> = HashMap::new()\n    println(\"${strsim::tally(m)}\")\n}\n";
    let p = Project::new("", main, Some(surface));
    let o = p.wj(&["run", "--interpret"]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    let t = text(&o);
    assert!(t.contains("not interpreted yet: call into Rust crate dependency `strsim` (`strsim::tally`) [deps.rust_crate]"), "{t}");
}

// ---- wj eject ----

#[test]
fn eject_keeps_the_crate_line_and_borrowing_call_sites() {
    let p = Project::new("", MAIN, Some(SURFACE));
    let out = p.root.join("ejected");
    let o = p.wj(&["eject", ".", "-o", out.to_str().unwrap()]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(read(&out.join("Cargo.toml")).contains("strsim = \"0.11\""));
    assert!(read(&out.join("main.rs")).contains("strsim::levenshtein(&a, &b)"));
    assert!(!out.join("rust").exists(), "surfaces are Windjammer-side only");
}

// ---- end to end: a real crate from the local cargo cache ----

#[test]
fn wj_build_produces_a_working_binary_with_a_real_crate() {
    let p = Project::new("", MAIN, Some(SURFACE));
    let (o, out) = p.transpile();
    assert!(o.status.success(), "{}", text(&o));
    let target = std::env::var_os("WJ_INTEROP_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| p.root.join("target"));
    let run = Command::new("cargo")
        .args(["run", "--quiet", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_PROFILE_RELEASE_LTO", "false")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "{}", text(&run));
    assert!(stdout.contains("distance 3"), "{stdout}");
    // Jaro("martha", "marhta") = 0.944.. (Winkler's paper)
    assert!(stdout.contains("jaro 0.944"), "{stdout}");
}

// ---- Windjammer path dependencies are never Rust crates (direct, `../pkg/build`, transitive) ----

/// `app` -> `pkg_a` (path `../pkg_a/build`) -> `pkg_b` (path `../pkg_b/build`), and `app` -> `pkg_b` directly.
/// Each `build/` holds what `wj build` leaves there: a Cargo crate transpiled from the package. Its Rust
/// signature is generic, so a surface generated from it (the bug) would reject `pkg_b::f` with WJ0050.
fn windjammer_path_packages() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let w = |rel: &str, text: &str| {
        let p = tmp.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    for (pkg, deps, body) in [
        ("pkg_b", "", "pub fn f(x: int) -> int {\n    x + 1\n}\n"),
        ("pkg_a", "pkg_b = { path = \"../pkg_b/build\" }\n", "use pkg_b\n\npub fn g(x: int) -> int {\n    pkg_b::f(x) * 2\n}\n"),
    ] {
        w(&format!("{pkg}/wj.toml"), &format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n\n[lib]\n\n[dependencies]\n{deps}"));
        w(&format!("{pkg}/src/lib.wj"), body);
        w(&format!("{pkg}/build/Cargo.toml"), &format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"));
        w(&format!("{pkg}/build/src/lib.rs"), "pub fn f<T: Into<i64>>(x: T) -> i64 { x.into() + 1 }\npub fn g<T: Into<i64>>(x: T) -> i64 { x.into() * 2 }\n");
    }
    w(
        "app/wj.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\npkg_a = { path = \"../pkg_a/build\" }\npkg_b = { path = \"../pkg_b/build\" }\n",
    );
    w("app/src/main.wj", "use pkg_a\nuse pkg_b\n\nfn main() {\n    println(\"${pkg_a::g(1)} ${pkg_b::f(2)}\")\n}\n");
    let app = tmp.path().join("app");
    (tmp, app)
}

#[test]
fn windjammer_path_dependencies_are_not_rust_crates() {
    let (tmp, app) = windjammer_path_packages();
    let run = |dir: &Path, args: &[&str]| {
        let o = Command::new(WJ).args(args).current_dir(dir).env("NO_COLOR", "1").output().unwrap();
        text(&o)
    };
    let out = app.join("out");
    let build = run(&app, &["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
    assert!(!build.contains("WJ0050") && !build.contains("signature surface"), "{build}");
    assert!(!app.join(".wj-cache/rust-surfaces/pkg_b").exists() && !app.join(".wj-cache/rust-surfaces/pkg_a").exists());
    for dir in [app.clone(), tmp.path().join("pkg_a")] {
        let check = run(&dir, &["check", "."]);
        assert!(!check.contains("W0014") && !check.contains("WJ0050"), "{}: {check}", dir.display());
    }
    // the same project with a real Rust crate next to it still classifies that one as Rust
    let manifest = std::fs::read_to_string(app.join("wj.toml")).unwrap() + "strsim = \"0.11\"\n";
    std::fs::write(app.join("wj.toml"), manifest).unwrap();
    std::fs::write(app.join("src/main.wj"), "use pkg_b\nuse strsim\n\nfn main() {\n    println(\"${pkg_b::f(2)} ${strsim::levenshtein(\"a\", \"b\")}\")\n}\n").unwrap();
    std::fs::remove_dir_all(app.join(".wj-cache")).ok();
    let check = run(&app, &["check", "."]);
    assert!(check.contains("W0014") && check.contains("`strsim`") && !check.contains("`pkg_b`"), "{check}");
}
