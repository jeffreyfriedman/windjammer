// consumes: src/cli/build_caps.rs, src/ir/effect_facts/install.rs, crates/windjammer-runtime/src/{caps.rs,cap.rs}, std/runtime/{js,go}
//! Compile-time enforcement and run-time guards in the compiled and portable backends (ADR-0021 stage E2), end to
//! end through `wj build`:
//!
//! * a call the compiler can prove inside the grant carries NO guard (zero cost);
//! * a computed path, host or name makes the program install its grant at the start of `main`;
//! * a call outside the grant (or a dependency gap) stops the build before any code is generated;
//! * the JavaScript and Go programs enforce the same grant and stop with exit status 126.
//!
//! The Rust program itself is compiled and run only with `WJ_EFFECTS_COMPILED=1` (it builds the runtime crate:
//! about 40 s warm, much longer cold).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn wj(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(dir).output().expect("run wj")
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn project(main: &str, caps: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!("wj_compiled_{}_{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("data")).unwrap();
    let root = std::fs::canonicalize(&root).unwrap();
    std::fs::write(root.join("wj.toml"), format!("[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n{caps}")).unwrap();
    std::fs::write(root.join("src/main.wj"), main).unwrap();
    std::fs::write(root.join("data/a.txt"), "hello-from-data").unwrap();
    root
}

const CAPS: &str = "[capabilities]\nfs.read = [\"./data/**\"]\n";
const PROVEN: &str = "use std::fs\nfn main() {\n    println(fs::read_to_string(\"data/a.txt\").unwrap_or(\"\"))\n}\n";
const COMPUTED: &str = "use std::fs\nfn load(name: string) -> string {\n    fs::read_to_string(\"data/\" + name).unwrap_or(\"\")\n}\nfn main() {\n    println(load(\"a.txt\"))\n}\n";
const ESCAPE: &str = "use std::fs\nfn load(name: string) -> string {\n    fs::read_to_string(\"data/\" + name).unwrap_or(\"\")\n}\nfn main() {\n    println(\"start\")\n    println(load(\"../wj.toml\"))\n}\n";

fn emitted_rust(main: &str, caps: &str) -> (Output, String) {
    let p = project(main, caps);
    let out = wj(&["build", "--no-cargo", "-o", "out", "."], &p);
    let rs = std::fs::read_to_string(p.join("out/main.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&p);
    (out, rs)
}

#[test]
fn a_program_proven_inside_its_grant_carries_no_guard() {
    let (out, rs) = emitted_rust(PROVEN, CAPS);
    assert!(out.status.success(), "{}", text(&out));
    assert!(!rs.contains("caps::install"), "zero guards for a proven program:\n{rs}");
}

#[test]
fn a_computed_path_installs_the_grant_first_thing_in_main() {
    let (out, rs) = emitted_rust(COMPUTED, CAPS);
    assert!(out.status.success(), "{}", text(&out));
    let main = rs.split("fn main()").nth(1).expect("main in the output");
    let first = main.lines().find(|l| !l.trim().is_empty() && !l.contains("fn main")).unwrap_or("");
    assert!(first.contains("windjammer_runtime::caps::install(") && first.contains(r#"("fs.read", &["./data/**"])"#), "first statement of main: {first}\n{rs}");
    assert!(first.contains(r#"root: ".""#), "the root is the working directory, not a machine path: {first}");
}

#[test]
fn a_capability_value_makes_the_program_install_its_grant() {
    let main = "use std::cap\nfn main() {\n    let files = cap::fs().under(\"data\").read_only()\n    println(files.read_to_string(\"data/a.txt\").unwrap_or(\"\"))\n}\n";
    let (out, rs) = emitted_rust(main, CAPS);
    assert!(out.status.success(), "{}", text(&out));
    assert!(rs.contains("caps::install("), "minting reads the installed grant, so it must exist:\n{rs}");
    assert!(rs.contains("cap::fs().under(") && rs.contains(".read_only()"), "{rs}");
}

#[test]
fn a_call_outside_the_grant_stops_the_build_before_any_code_is_generated() {
    let main = "use std::fs\nfn main() {\n    println(fs::read_to_string(\"/etc/hosts\").unwrap_or(\"\"))\n}\n";
    let (out, rs) = emitted_rust(main, CAPS);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("WJ0041") && text(&out).contains("capability error"), "{}", text(&out));
    assert!(rs.is_empty(), "no code is generated for a rejected program:\n{rs}");
}

#[test]
fn a_project_without_capabilities_is_built_exactly_as_before() {
    let (out, rs) = emitted_rust(COMPUTED, "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(!rs.contains("caps::install"), "no [capabilities]: nothing to enforce:\n{rs}");
}

#[test]
fn compiled_programs_cannot_hold_a_dependency_to_a_narrower_grant_so_the_build_says_so() {
    let p = project("use json5\nfn main() {\n    println(json5::load(\"a.txt\"))\n}\n", "");
    let lib = p.parent().unwrap().join(format!("{}_json5", p.file_name().unwrap().to_string_lossy()));
    std::fs::create_dir_all(lib.join("src")).unwrap();
    std::fs::write(lib.join("wj.toml"), "[package]\nname = \"json5\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(lib.join("src/lib.wj"), "use std::fs\npub fn load(name: string) -> string {\n    fs::read_to_string(\"data/\" + name).unwrap_or(\"\")\n}\n").unwrap();
    let caps = format!("[dependencies]\njson5 = {{ path = \"{}\" }}\n\n[capabilities]\nfs.read = [\"./**\"]\n\n[capabilities.dependencies.json5]\nfs.read = [\"./data/**\"]\n", lib.display());
    std::fs::write(p.join("wj.toml"), format!("[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n{caps}")).unwrap();
    let out = wj(&["build", "--no-cargo", "-o", "out", "."], &p);
    let t = text(&out);
    let _ = std::fs::remove_dir_all(&p);
    let _ = std::fs::remove_dir_all(&lib);
    assert!(!out.status.success(), "{t}");
    assert!(t.contains("dependency `json5` uses a computed `fs.read` resource") && t.contains("capability value"), "{t}");
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg(if tool == "go" { "version" } else { "--version" }).output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn javascript_program_enforces_the_same_grant() {
    if !have("node") {
        eprintln!("node not installed: skipping");
        return;
    }
    for (main, want_code, want_out) in [(COMPUTED, 0, "hello-from-data"), (ESCAPE, 126, "capability denied: fs.read(./wj.toml)")] {
        let p = project(main, CAPS);
        let b = wj(&["build", "-t", "javascript", "-o", "outjs", "src/main.wj"], &p);
        assert!(b.status.success(), "{}", text(&b));
        let js = std::fs::read_to_string(p.join("outjs/main.mjs")).unwrap();
        assert!(js.contains("$caps_install("), "{js}");
        let run = Command::new("node").arg("outjs/main.mjs").current_dir(&p).output().unwrap();
        let t = text(&run);
        let _ = std::fs::remove_dir_all(&p);
        assert_eq!(run.status.code(), Some(want_code), "{t}");
        assert!(t.contains(want_out), "{t}");
        if want_code == 126 {
            assert!(!t.contains("[package]"), "the file must not have been read: {t}");
        }
    }
}

#[test]
fn go_program_enforces_the_same_grant() {
    if !have("go") {
        eprintln!("go not installed: skipping");
        return;
    }
    for (main, want_code, want_out) in [(COMPUTED, 0, "hello-from-data"), (ESCAPE, 126, "capability denied: fs.read(./wj.toml)")] {
        let p = project(main, CAPS);
        let b = wj(&["build", "-t", "go", "-o", "outgo", "src/main.wj"], &p);
        assert!(b.status.success(), "{}", text(&b));
        let go = std::fs::read_to_string(p.join("outgo/main.go")).unwrap();
        assert!(go.contains("wjstd_caps_install("), "{go}");
        // run from the project directory: relative paths and ./data/** are anchored at the working directory
        let bin = p.join("outgo/prog");
        let c = Command::new("go").args(["build", "-o"]).arg(&bin).arg(".").current_dir(p.join("outgo")).output().unwrap();
        assert!(c.status.success(), "{}", text(&c));
        let run = Command::new(&bin).current_dir(&p).output().unwrap();
        let t = text(&run);
        let _ = std::fs::remove_dir_all(&p);
        assert_eq!(run.status.code(), Some(want_code), "{t}");
        assert!(t.contains(want_out), "{t}");
    }
}

/// The compiled Rust program, built with cargo and run. Heavy: opt in with WJ_EFFECTS_COMPILED=1.
#[test]
fn compiled_rust_program_enforces_the_grant_and_exits_126() {
    if std::env::var("WJ_EFFECTS_COMPILED").is_err() {
        eprintln!("set WJ_EFFECTS_COMPILED=1 to build and run the compiled Rust program");
        return;
    }
    for (main, want_code, want_out) in [(COMPUTED, 0, "hello-from-data"), (ESCAPE, 126, "capability denied: fs.read(./wj.toml)")] {
        let p = project(main, CAPS);
        let b = wj(&["build", "-o", "outc", "."], &p);
        assert!(b.status.success(), "{}", text(&b));
        let bin = std::fs::read_dir(p.join("outc/target/debug")).unwrap().flatten().map(|e| e.path()).find(|f| f.file_name().is_some_and(|n| n == "p")).expect("compiled binary");
        let run = Command::new(bin).current_dir(&p).output().unwrap();
        let t = text(&run);
        let _ = std::fs::remove_dir_all(&p);
        assert_eq!(run.status.code(), Some(want_code), "{t}");
        assert!(t.contains(want_out), "{t}");
    }
}
