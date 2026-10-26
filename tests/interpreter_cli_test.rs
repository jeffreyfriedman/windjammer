#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! End-to-end tests of the interpreted dev loop through the real `wj` binary:
//! `wj run --interpret` (file and multi-file project), `wj test --interpret`, `wj repl`,
//! exit codes, diagnostics, and the guarantee that no cargo/rustc is ever spawned.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write(dir: &Path, name: &str, content: &str) {
    let p = dir.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

fn wj(args: &[&str], cwd: &Path) -> Output {
    Command::new(WJ).args(args).current_dir(cwd).output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

#[test]
fn run_interpret_prints_only_program_output() {
    let d = TempDir::new().unwrap();
    write(d.path(), "hello.wj", "fn main() {\n    println(\"hello {}\", 1 + 2)\n}\n");
    let o = wj(&["run", "--interpret", "hello.wj"], d.path());
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(out(&o), "hello 3\n", "no banner or decoration may precede program output");
}

#[test]
fn interpreted_run_never_spawns_cargo_or_rustc() {
    // Put shims named cargo/rustc/rustup first on PATH; if the interpreter ever execs one it
    // leaves a marker file. The program must still run.
    let d = TempDir::new().unwrap();
    let shims = d.path().join("shims");
    fs::create_dir_all(&shims).unwrap();
    let marker = d.path().join("SPAWNED");
    for tool in ["cargo", "rustc", "rustup", "cc", "gcc"] {
        let p = shims.join(tool);
        fs::write(&p, format!("#!/bin/sh\necho {} >> {}\nexit 97\n", tool, marker.display())).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    write(d.path(), "p.wj", "fn main() {\n    let v = vec![1, 2, 3]\n    println(\"{}\", v.len())\n}\n");
    let o = Command::new(WJ)
        .args(["run", "--interpret", "p.wj"])
        .env("PATH", format!("{}:/usr/bin:/bin", shims.display()))
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(out(&o), "3\n");
    assert!(!marker.exists(), "interpreter spawned a toolchain process: {:?}", fs::read_to_string(&marker));
}

#[test]
fn interpreted_tests_never_spawn_cargo_or_rustc() {
    let d = TempDir::new().unwrap();
    let shims = d.path().join("shims");
    fs::create_dir_all(&shims).unwrap();
    let marker = d.path().join("SPAWNED");
    for tool in ["cargo", "rustc"] {
        let p = shims.join(tool);
        fs::write(&p, format!("#!/bin/sh\necho {} >> {}\nexit 97\n", tool, marker.display())).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    write(d.path(), "m_test.wj", "@test\nfn test_ok() {\n    assert_eq(1 + 1, 2)\n}\n");
    let o = Command::new(WJ)
        .args(["test", "--interpret"])
        .env("PATH", format!("{}:/usr/bin:/bin", shims.display()))
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}{}", out(&o), err(&o));
    assert!(!marker.exists(), "wj test --interpret spawned a toolchain process");
}

#[test]
fn panic_exits_101_with_source_span_and_backtrace() {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "boom.wj",
        "fn inner(n: int) -> int {\n    let v = vec![1, 2, 3]\n    v[n]\n}\nfn outer() -> int {\n    inner(9)\n}\nfn main() {\n    println(\"before\")\n    println(\"{}\", outer())\n}\n",
    );
    let o = wj(&["run", "--interpret", "boom.wj"], d.path());
    assert_eq!(o.status.code(), Some(101), "{}", err(&o));
    assert_eq!(out(&o), "before\n", "output before the panic must be flushed");
    let e = err(&o);
    assert!(e.contains("index out of bounds: the len is 3 but the index is 9"), "{}", e);
    // parser locations are recorded after the node is consumed, so the span may name the next line
    assert!(e.contains("boom.wj:3") || e.contains("boom.wj:4"), "panic must point at the failing line: {}", e);
    assert!(e.contains("inner") && e.contains("outer") && e.contains("main"), "backtrace frames: {}", e);
}

#[test]
fn process_exit_code_is_propagated() {
    let d = TempDir::new().unwrap();
    write(d.path(), "e.wj", "use std::process\nfn main() {\n    println(\"bye\")\n    process::exit(7)\n}\n");
    let o = wj(&["run", "--interpret", "e.wj"], d.path());
    assert_eq!(o.status.code(), Some(7), "{}", err(&o));
    assert_eq!(out(&o), "bye\n");
}

#[test]
fn unsupported_feature_is_a_clear_diagnostic_not_wrong_output() {
    let d = TempDir::new().unwrap();
    write(d.path(), "a.wj", "fn main() {\n    println(\"start\")\n    thread {\n        println(\"in thread\")\n    }\n}\n");
    let o = wj(&["run", "--interpret", "a.wj"], d.path());
    assert_eq!(o.status.code(), Some(2), "{}", err(&o));
    let e = err(&o);
    assert!(e.contains("unsupported in the interpreter"), "{}", e);
    assert!(e.contains("wj run"), "diagnostic should point at the compiled path: {}", e);
}

#[test]
fn multi_file_project_runs_from_a_directory() {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "src/main.wj",
        "use math\nuse shapes::Circle\nfn main() {\n    println(\"{}\", math::double(21))\n    let c = Circle { r: 2 }\n    println(\"{}\", c.area())\n    println(\"{}\", math::square(5))\n}\n",
    );
    write(d.path(), "src/math.wj", "pub fn double(x: int) -> int {\n    x * 2\n}\npub fn square(x: int) -> int {\n    helper(x) * x\n}\nfn helper(x: int) -> int {\n    x\n}\n");
    write(d.path(), "src/shapes.wj", "pub struct Circle {\n    pub r: int,\n}\nimpl Circle {\n    pub fn area(self) -> int {\n        3 * self.r * self.r\n    }\n}\n");
    let o = wj(&["run", "--interpret", "."], d.path());
    assert!(o.status.success(), "{}{}", out(&o), err(&o));
    assert_eq!(out(&o), "42\n12\n25\n");
}

#[test]
fn imported_names_do_not_collide_across_modules() {
    let d = TempDir::new().unwrap();
    write(d.path(), "main.wj", "use a\nuse b\nfn main() {\n    println(\"{} {}\", a::f(), b::f())\n}\n");
    write(d.path(), "a.wj", "pub fn f() -> int {\n    1\n}\n");
    write(d.path(), "b.wj", "pub fn f() -> int {\n    2\n}\n");
    let o = wj(&["run", "--interpret", "main.wj"], d.path());
    assert!(o.status.success(), "{}{}", out(&o), err(&o));
    assert_eq!(out(&o), "1 2\n");
}

#[test]
fn test_interpret_runs_test_functions_and_reports_failures() {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "calc_test.wj",
        "fn add(a: int, b: int) -> int {\n    a + b\n}\n@test\nfn test_add() {\n    assert_eq(add(2, 3), 5)\n}\n@test\nfn test_add_negative() {\n    assert_eq(add(-2, -3), -5)\n}\n@test\nfn test_broken() {\n    assert_eq(add(1, 1), 3)\n}\n@test_cases([(1, 2, 3), (10, 20, 30)])\nfn test_cases_add(a: int, b: int, want: int) {\n    assert_eq(add(a, b), want)\n}\nfn helper_not_a_test() {\n    panic(\"must not run\")\n}\n",
    );
    let o = wj(&["test", "--interpret"], d.path());
    let s = out(&o);
    assert_eq!(o.status.code(), Some(1), "{}{}", s, err(&o));
    assert!(s.contains("test_add ... ok"), "{}", s);
    assert!(s.contains("test_add_negative ... ok"), "{}", s);
    assert!(s.contains("test_cases_add ... ok"), "{}", s);
    assert!(s.contains("test_broken ... FAILED"), "{}", s);
    assert!(s.contains("left: 2") && s.contains("right: 3"), "assertion message: {}", s);
    assert!(s.contains("3 passed; 1 failed"), "{}", s);
    assert!(!s.contains("must not run"));
}

#[test]
fn test_interpret_filter_selects_tests() {
    let d = TempDir::new().unwrap();
    write(d.path(), "t_test.wj", "@test\nfn test_alpha() {\n    assert(true)\n}\n@test\nfn test_beta() {\n    panic(\"beta ran\")\n}\n");
    let o = wj(&["test", "--interpret", "--filter", "alpha"], d.path());
    assert!(o.status.success(), "{}{}", out(&o), err(&o));
    assert!(out(&o).contains("test_alpha ... ok") && !out(&o).contains("test_beta"));
}

#[test]
fn repl_keeps_state_between_inputs() {
    let mut child = Command::new(WJ).arg("repl").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(b"let x = 20\nfn twice(n: int) -> int {\n    n * 2\n}\ntwice(x) + 2\nlet v = vec![1, 2, 3]\nv.len()\n\"hi\"\nexit\n")
            .unwrap();
    }
    let o = child.wait_with_output().unwrap();
    let s = out(&o);
    assert!(o.status.success(), "{}", err(&o));
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines, vec!["42", "3", "\"hi\""], "stderr: {}", err(&o));
}

#[test]
fn repl_reports_errors_and_continues() {
    let mut child = Command::new(WJ).arg("repl").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.as_mut().unwrap().write_all(b"let a: i32 = 2147483647\na + 1\n1 + 1\n").unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(out(&o).lines().collect::<Vec<_>>(), vec!["2"]);
    assert!(err(&o).contains("attempt to add with overflow"), "{}", err(&o));
}

#[test]
fn startup_is_fast_enough_for_a_dev_loop() {
    // The point of the interpreter: no toolchain round trip. A trivial program must start and
    // finish far below a cargo build; keep a generous bound so a loaded CI machine does not flake.
    let d = TempDir::new().unwrap();
    write(d.path(), "t.wj", "fn main() {\n    println(\"x\")\n}\n");
    let start = std::time::Instant::now();
    let o = wj(&["run", "--interpret", "t.wj"], d.path());
    assert!(o.status.success());
    assert!(start.elapsed() < std::time::Duration::from_secs(5), "took {:?}", start.elapsed());
}

/// A script defines its own flags: everything after the script path reaches the program, including a first
/// argument that starts with `--` (the program's own `--name value` flags), and `env::args()` is indexable.
#[test]
fn run_passes_program_flags_after_the_path_and_args_is_a_vec() {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "argv.wj",
        "use std::env\n\nfn main() {\n    let args = env.args()\n    println(\"${args.len()} ${args[1]} ${args[2]} ${args[3]}\")\n}\n",
    );
    let o = wj(&["run", "--interpret", "argv.wj", "--name", "value", "positional"], d.path());
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(out(&o), "4 --name value positional\n");
    // wj's own flags before the path still belong to wj
    let o = wj(&["run", "argv.wj", "--interpret", "a", "b", "c"], d.path());
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(out(&o), "4 a b c\n");
}
