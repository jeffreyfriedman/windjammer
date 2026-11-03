//! ADR-0026 generator: `wj rust-surface`, generated surfaces in `.wj-cache`, precedence, staleness, the
//! tracked-generated-files gate. Run: `cargo test --release --test rust_surface_generator`.
//!
//! Oracles are independent of the generator: the hand-written strsim surface of `tests/rust_crate_interop.rs`,
//! the crate sources in the local Cargo registry (counted with plain text matching), the real crate's output.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// The hand-written surface `tests/rust_crate_interop.rs` uses (signatures checked against rustc there).
const ORACLE: &[&str] = &[
    "pub fn levenshtein(a: string, b: string) -> usize",
    "pub fn jaro(a: string, b: string) -> f64",
    "pub fn osa_distance(a: string, b: string) -> usize",
];

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

struct Project {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new(deps: &str, main: &str) -> Self {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("app");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("wj.toml"), format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\n{deps}")).unwrap();
        std::fs::write(root.join("src/main.wj"), main).unwrap();
        Project { _tmp: tmp, root }
    }

    fn strsim(main: &str) -> Self {
        Self::new("strsim = \"0.11\"\n", main)
    }

    fn wj(&self, args: &[&str]) -> Output {
        self.wj_env(args, &[])
    }

    fn wj_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut c = Command::new(WJ);
        c.args(args).current_dir(&self.root).env("NO_COLOR", "1");
        for (k, v) in env {
            c.env(k, v);
        }
        c.output().unwrap()
    }

    fn write(&self, rel: &str, text: &str) {
        let p = self.root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn cache(&self, krate: &str) -> PathBuf {
        self.root.join(".wj-cache/rust-surfaces").join(krate).join("surface.wji")
    }
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

/// The newest `<name>-<version>` source directory in the local Cargo registry that starts with `prefix`.
fn registry_crate(prefix: &str) -> PathBuf {
    let home = std::env::var_os("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap()).join(".cargo"));
    let mut found: Vec<PathBuf> = std::fs::read_dir(home.join("registry/src"))
        .unwrap()
        .flatten()
        .flat_map(|idx| std::fs::read_dir(idx.path()).into_iter().flatten().flatten().map(|e| e.path()))
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix))
        .collect();
    found.sort();
    found.pop().unwrap_or_else(|| panic!("{prefix}* is not in the local Cargo registry (run `cargo fetch` for a project that uses it)"))
}

/// Public free functions at the top of a crate root, by plain text (`pub fn name` at column 0, outside
/// `#[cfg(test)]` modules, which are indented).
fn top_level_pub_fns(lib: &Path) -> Vec<String> {
    std::fs::read_to_string(lib)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix("pub fn "))
        .map(|l| l.split(|c: char| c == '(' || c == '<').next().unwrap().to_string())
        .collect()
}

fn declared(surface: &str) -> Vec<String> {
    surface.lines().filter(|l| l.starts_with("pub fn ")).map(str::to_string).collect()
}

fn skipped(surface: &str) -> Vec<(String, String)> {
    surface
        .lines()
        .filter_map(|l| l.strip_prefix("// skipped "))
        .map(|l| {
            let (n, w) = l.split_once(": ").unwrap();
            (n.to_string(), w.to_string())
        })
        .collect()
}

// ---- generation against the oracle ----

#[test]
fn strsim_matches_the_hand_written_surface() {
    let p = Project::strsim(MAIN);
    let o = p.wj(&["rust-surface", "strsim"]);
    assert!(o.status.success(), "{}", text(&o));
    let s = stdout(&o);
    let lines = declared(&s);
    for want in ORACLE {
        assert!(lines.iter().any(|l| l == want), "missing `{want}` in\n{s}");
    }
    let src = registry_crate("strsim-0.11.");
    let version = src.file_name().unwrap().to_string_lossy().trim_start_matches("strsim-").to_string();
    assert!(s.contains(&format!("// crate: strsim {version}\n")), "{s}");
    // curated pure crate: every declared function is @pure
    assert!(s.contains("@pure\npub fn levenshtein("), "{s}");
    assert_eq!(s.matches("@pure\n").count(), lines.len(), "{s}");
    // honest: every top-level `pub fn` of the crate is declared or skipped, nothing is silently dropped
    let all = top_level_pub_fns(&src.join("src/lib.rs"));
    let skip = skipped(&s);
    for f in &all {
        let decl = lines.iter().any(|l| l.starts_with(&format!("pub fn {f}(")));
        let sk = skip.iter().any(|(n, _)| n == f);
        assert!(decl ^ sk, "`{f}` must be declared xor skipped:\n{s}");
    }
    assert_eq!(lines.len() + skip.len(), all.len(), "{s}");
    assert!(skip.iter().any(|(n, w)| n == "generic_levenshtein" && w.starts_with("generic over `")), "{skip:?}");
    assert!(skip.iter().any(|(n, w)| n == "hamming" && w.contains("StrSimError")), "{skip:?}");
    // the cache copy has every function and the cache marker
    let cached = std::fs::read_to_string(p.cache("strsim")).unwrap();
    assert!(cached.starts_with("// @generated by `wj rust-surface`"), "{cached}");
    assert_eq!(cached, s);
}

#[test]
fn generics_and_trait_bounds_are_listed_not_guessed() {
    let p = Project::new("hex = \"0.4\"\n", "fn main() {}\n");
    let o = p.wj(&["rust-surface", "hex"]);
    assert!(o.status.success(), "{}", text(&o));
    let s = stdout(&o);
    let src = registry_crate("hex-0.4.");
    let all = top_level_pub_fns(&src.join("src/lib.rs"));
    assert_eq!(all.len(), 5, "oracle drift: {all:?}");
    assert!(declared(&s).is_empty(), "every hex function is generic over AsRef<[u8]>:\n{s}");
    let skip = skipped(&s);
    for f in &all {
        let reason = &skip.iter().find(|(n, _)| n == f).unwrap_or_else(|| panic!("`{f}` not listed:\n{s}")).1;
        assert!(reason.contains("generic over `T: AsRef<[u8]>`"), "{f}: {reason}");
    }
    // asking for one explains why instead of writing something wrong
    let o = p.wj(&["rust-surface", "hex", "--fn", "encode", "--write"]);
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("`hex::encode` cannot be generated (generic over `T: AsRef<[u8]>`)"), "{}", text(&o));
    assert!(!p.root.join("rust/hex.wji").exists());
}

#[test]
fn output_is_deterministic_across_runs_and_thread_counts() {
    let p = Project::new("strsim = \"0.11\"\nhex = \"0.4\"\n", "fn main() {}\n");
    for krate in ["strsim", "hex"] {
        let a = p.wj_env(&["rust-surface", krate], &[("RAYON_NUM_THREADS", "1")]);
        let cache_a = std::fs::read(p.cache(krate)).unwrap();
        std::fs::remove_dir_all(p.root.join(".wj-cache")).unwrap();
        let b = p.wj_env(&["rust-surface", krate], &[("RAYON_NUM_THREADS", "8")]);
        let cache_b = std::fs::read(p.cache(krate)).unwrap();
        assert!(a.status.success() && b.status.success(), "{}{}", text(&a), text(&b));
        assert_eq!(a.stdout, b.stdout, "{krate}: stdout differs");
        assert_eq!(cache_a, cache_b, "{krate}: cache file differs");
    }
}

// ---- precedence, --write ----

#[test]
fn hand_written_surface_wins_over_the_generated_one() {
    let main = "use strsim\nfn main() {\n    println(\"${strsim::jaro(\"a\", \"b\")}\")\n}\n";
    let p = Project::strsim(main);
    assert!(p.wj(&["rust-surface", "strsim"]).status.success());
    let o = p.wj(&["check", "."]);
    assert!(o.status.success(), "generated surface declares jaro: {}", text(&o));
    p.write("rust/strsim.wji", "@pure\npub fn levenshtein(a: string, b: string) -> usize\n");
    let o = p.wj(&["check", "."]);
    assert!(!o.status.success() && text(&o).contains("error[WJ0050]") && text(&o).contains("strsim::jaro"), "{}", text(&o));
    // --write never overwrites a hand-written file
    let o = p.wj(&["rust-surface", "strsim", "--write"]);
    assert!(!o.status.success() && text(&o).contains("hand-written"), "{}", text(&o));
    assert!(std::fs::read_to_string(p.root.join("rust/strsim.wji")).unwrap().starts_with("@pure\npub fn levenshtein"));
}

#[test]
fn write_pins_an_editable_copy_that_is_source() {
    let p = Project::strsim(MAIN);
    let o = p.wj(&["rust-surface", "strsim", "--write", "--fn", "levenshtein", "--fn", "jaro"]);
    assert!(o.status.success(), "{}", text(&o));
    let pinned = std::fs::read_to_string(p.root.join("rust/strsim.wji")).unwrap();
    assert!(!pinned.starts_with("// @generated"), "a pinned copy must not carry the cache marker:\n{pinned}");
    assert_eq!(declared(&pinned), ["pub fn jaro(a: string, b: string) -> f64", "pub fn levenshtein(a: string, b: string) -> usize"]);
    let o = p.wj(&["check", "."]);
    assert!(o.status.success(), "{}", text(&o));
    // the pinned copy is what check uses: osa_distance is not in it although the cache has it
    let p2 = Project::strsim("use strsim\nfn main() {\n    println(\"${strsim::osa_distance(\"a\", \"b\")}\")\n}\n");
    std::fs::create_dir_all(p2.root.join("rust")).unwrap();
    std::fs::write(p2.root.join("rust/strsim.wji"), &pinned).unwrap();
    assert!(p2.wj(&["rust-surface", "strsim"]).status.success());
    let o = p2.wj(&["check", "."]);
    assert!(!o.status.success() && text(&o).contains("error[WJ0050]"), "{}", text(&o));
}

// ---- check: missing, skipped, stale ----

#[test]
fn check_offers_the_exact_command_when_no_surface_exists() {
    let p = Project::strsim(MAIN);
    let o = p.wj(&["check", "."]);
    let t = text(&o);
    assert!(t.contains("warning[W0014]") && t.contains("Rust crate `strsim` has no signature surface"), "{t}");
    assert!(t.contains("`wj rust-surface strsim`"), "{t}");
    assert!(t.contains("`wj rust-surface strsim --write --fn levenshtein --fn jaro`"), "{t}");
    assert!(t.contains("main.wj:6"), "{t}");
    // once generated, the warning is gone
    assert!(p.wj(&["rust-surface", "strsim"]).status.success());
    let t = text(&p.wj(&["check", "."]));
    assert!(!t.contains("W0014"), "{t}");
}

#[test]
fn calling_a_skipped_function_says_why() {
    let main = "use strsim\nfn main() {\n    println(\"${strsim::generic_levenshtein(\"a\", \"b\")}\")\n}\n";
    let p = Project::strsim(main);
    assert!(p.wj(&["rust-surface", "strsim"]).status.success());
    let o = p.wj(&["check", "."]);
    let t = text(&o);
    assert!(!o.status.success() && t.contains("error[WJ0050]"), "{t}");
    assert!(t.contains("the generator skipped it (generic over `"), "{t}");
}

fn lock_with(version: &str) -> String {
    format!("version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"strsim\"\nversion = \"{version}\"\n")
}

#[test]
fn stale_generated_surface_is_reported() {
    let p = Project::strsim(MAIN);
    let o = p.wj(&["rust-surface", "strsim", "--write"]);
    assert!(o.status.success(), "{}", text(&o));
    let pinned = std::fs::read_to_string(p.root.join("rust/strsim.wji")).unwrap();
    let current = pinned.lines().find_map(|l| l.strip_prefix("// crate: strsim ")).unwrap().to_string();
    // the lock file names the version the surface was generated from: no warning
    p.write("build/Cargo.lock", &lock_with(&current));
    let t = text(&p.wj(&["check", "."]));
    assert!(!t.contains("W0015"), "{t}");
    // the lock moved on: warning at the header line, with the command
    let old = pinned.replace(&format!("// crate: strsim {current}"), "// crate: strsim 0.10.0");
    p.write("rust/strsim.wji", &old);
    let t = text(&p.wj(&["check", "."]));
    assert!(t.contains("warning[W0015]"), "{t}");
    assert!(t.contains(&format!("generated from strsim 0.10.0, older than the locked {current}")), "{t}");
    assert!(t.contains("strsim.wji:3"), "{t}");
    assert!(t.contains("wj rust-surface strsim --write"), "{t}");
}

#[test]
fn build_regenerates_a_stale_cache_copy() {
    let p = Project::strsim(MAIN);
    assert!(p.wj(&["rust-surface", "strsim"]).status.success());
    let fresh = std::fs::read_to_string(p.cache("strsim")).unwrap();
    let current = fresh.lines().find_map(|l| l.strip_prefix("// crate: strsim ")).unwrap().to_string();
    std::fs::write(p.cache("strsim"), fresh.replace(&format!("// crate: strsim {current}"), "// crate: strsim 0.10.0")).unwrap();
    let out = p.root.join("out");
    let o = p.wj(&["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("note: generated the signature surface of Rust crate `strsim`"), "{}", text(&o));
    assert_eq!(std::fs::read_to_string(p.cache("strsim")).unwrap(), fresh);
}

// ---- end to end: no hand-written surface at all ----

#[test]
fn wj_build_without_any_surface_produces_a_working_binary() {
    let p = Project::strsim(MAIN);
    assert!(!p.root.join("rust").exists());
    let out = p.root.join("out");
    let o = p.wj(&["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
    assert!(o.status.success(), "{}", text(&o));
    let notes: Vec<&str> = std::str::from_utf8(&o.stderr).unwrap().lines().filter(|l| l.starts_with("note: generated")).collect();
    assert_eq!(notes.len(), 1, "one line per generated crate: {}", text(&o));
    assert!(p.cache("strsim").is_file());
    assert!(!p.root.join("rust").exists(), "nothing is written outside the cache");
    let rs = std::fs::read_to_string(out.join("main.rs")).unwrap();
    assert!(rs.contains("strsim::levenshtein(&a, &b)"), "{rs}");
    // a second build finds the cache fresh and says nothing
    let o = p.wj(&["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
    assert!(o.status.success() && !text(&o).contains("note: generated"), "{}", text(&o));
    let target = std::env::var_os("WJ_INTEROP_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| p.root.join("target"));
    let run = Command::new("cargo")
        .args(["run", "--quiet", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_PROFILE_RELEASE_LTO", "false")
        .output()
        .unwrap();
    let so = stdout(&run);
    assert!(run.status.success(), "{}", text(&run));
    assert!(so.contains("distance 3"), "{so}");
    assert!(so.contains("jaro 0.944"), "{so}");
}

// ---- repository gate: generated files are never committed ----

fn gate(repo: &Path) -> (Option<i32>, String) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/tracked_generated_gate.wj");
    let o = Command::new(WJ)
        .args(["run", "--interpret", "--allow", "process=git", "--allow", "fs.read=**"])
        .arg(script)
        .arg(repo)
        .output()
        .unwrap();
    (o.status.code(), text(&o))
}

#[test]
fn this_repository_tracks_no_generated_files() {
    let (code, t) = gate(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert_eq!(code, Some(0), "{t}");
    assert!(t.contains("none generated"), "{t}");
}

#[test]
fn gate_fails_on_planted_generated_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let r = tmp.path();
    let git = |args: &[&str]| assert!(Command::new("git").args(args).current_dir(r).output().unwrap().status.success());
    git(&["init", "-q"]);
    std::fs::create_dir_all(r.join("rust")).unwrap();
    std::fs::write(r.join("rust/pinned.wji"), "// generated by `wj rust-surface` (generator 1); this copy is source\n").unwrap();
    std::fs::write(r.join("main.wj"), "fn main() {}\n").unwrap();
    git(&["add", "-A"]);
    let (code, t) = gate(r);
    assert_eq!(code, Some(0), "a pinned copy is source: {t}");
    std::fs::write(r.join("rust/cached.wji"), "// @generated by `wj rust-surface` (generator 1); cache file\n").unwrap();
    std::fs::create_dir_all(r.join("app/.wj-cache/rust-surfaces/x")).unwrap();
    std::fs::write(r.join("app/.wj-cache/rust-surfaces/x/metadata.json"), "{}").unwrap();
    git(&["add", "-A", "-f"]);
    let (code, t) = gate(r);
    assert_eq!(code, Some(1), "{t}");
    assert!(t.contains("rust/cached.wji") && t.contains("app/.wj-cache/rust-surfaces/x/metadata.json"), "{t}");
    assert!(!t.contains("pinned.wji"), "{t}");
    // an empty listing is not a pass
    let empty = tempfile::TempDir::new().unwrap();
    let (code, _) = gate(empty.path());
    assert_ne!(code, Some(0));
}

// ---- every mapped shape compiles against a real crate (rustc is the oracle) ----

const SHAPES_LIB: &str = r#"
pub fn shout(s: &str) -> String { s.to_uppercase() }
pub fn bytes_len(b: &[u8]) -> usize { b.len() }
pub fn push_twice(v: &mut Vec<i64>, x: i64) { v.push(x); v.push(x); }
pub fn sum(v: &Vec<i64>) -> i64 { v.iter().sum() }
pub fn split(s: &str) -> Vec<String> { s.split(',').map(str::to_string).collect() }
pub fn parse(s: &str) -> Result<u32, String> { s.parse::<u32>().map_err(|e| e.to_string()) }
pub fn first(v: Vec<String>) -> Option<String> { v.into_iter().next() }
pub fn pair(a: u8, b: bool) -> (u8, bool) { (a, b) }
pub fn generic<T: Clone>(x: T) -> T { x }
"#;

const SHAPES_MAIN: &str = "\
use shapes

fn main() {
    println(shapes::shout(\"hi\"))
    let bytes: Vec<u8> = [1, 2, 3]
    println(\"${shapes::bytes_len(bytes)}\")
    let mut v: Vec<i64> = []
    shapes::push_twice(v, 21)
    println(\"${shapes::sum(v)}\")
    let parts = shapes::split(\"a,b\")
    println(\"${parts.len()}\")
    match shapes::parse(\"42\") {
        Ok(n) => println(\"parsed ${n}\"),
        Err(e) => println(\"error ${e}\"),
    }
    match shapes::first(parts) {
        Some(s) => println(\"first ${s}\"),
        None => println(\"none\"),
    }
    let (a, b) = shapes::pair(7, true)
    println(\"${a} ${b}\")
}
";

#[test]
fn mapped_shapes_compile_and_run_against_a_path_crate() {
    let p = Project::new("shapes = { path = \"../shapes\" }\n", SHAPES_MAIN);
    let crate_dir = p.root.parent().unwrap().join("shapes");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"shapes\"\nversion = \"0.3.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(crate_dir.join("src/lib.rs"), SHAPES_LIB).unwrap();
    let o = p.wj(&["rust-surface", "shapes"]);
    assert!(o.status.success(), "{}", text(&o));
    let s = stdout(&o);
    assert!(s.contains("// crate: shapes 0.3.0") && s.contains("every call is ffi(shapes)"), "{s}");
    assert_eq!(declared(&s).len(), 8, "{s}");
    assert_eq!(skipped(&s).len(), 1, "{s}");
    let out = p.root.join("out");
    let o = p.wj(&["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
    assert!(o.status.success(), "{}", text(&o));
    let target = std::env::var_os("WJ_INTEROP_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| p.root.join("target"));
    let run = Command::new("cargo")
        .args(["run", "--quiet", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_PROFILE_RELEASE_LTO", "false")
        .output()
        .unwrap();
    let rs = std::fs::read_to_string(out.join("main.rs")).unwrap_or_default();
    assert!(run.status.success(), "{}\n{rs}", text(&run));
    assert_eq!(stdout(&run), "HI\n3\n42\n2\nparsed 42\nfirst a\n7 true\n", "{rs}");
}
