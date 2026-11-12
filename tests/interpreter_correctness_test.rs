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

//! Interpreter correctness fixes found by earlier agents: each test runs a small program through
//! `wj run --interpret` and compares with what the std declaration / compiled runtime does.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Run {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run(dir: &std::path::Path, src: &str) -> Run {
    std::fs::write(dir.join("p.wj"), src).unwrap();
    let o = Command::new(WJ).current_dir(dir).args(["run", "--interpret", "--no-lint", "p.wj"]).output().unwrap();
    Run { stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into(), code: o.status.code().unwrap_or(-1) }
}

/// `std/fs.wj` declares `DirEntry { name, path, .. }` and `Metadata { size, .. }`: a field read gives what the
/// accessor method gives (it used to panic with `cannot project Field(name)`).
#[test]
fn dir_entry_name_field_reads_like_the_name_method() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(data.join("one.txt"), "12345").unwrap();
    let src = "use std::fs\n\nfn main() {\n    match fs::read_dir(\"data\") {\n        Ok(entries) => {\n            for e in entries {\n                println(\"field=${e.name} method=${e.name()} path=${e.path}\")\n                match e.metadata() {\n                    Ok(m) => println(\"size=${m.size} ${m.size()} file=${m.is_file}\"),\n                    Err(x) => println(\"meta error ${x}\"),\n                }\n            }\n        }\n        Err(x) => println(\"error ${x}\"),\n    }\n}\n";
    let r = run(dir.path(), src);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let line = r.stdout.lines().next().unwrap_or_default();
    assert!(line.starts_with("field=one.txt method=one.txt path="), "{}", r.stdout);
    assert!(r.stdout.contains("size=5 5 file=true"), "{}", r.stdout);
}

