//! Fixtures for the `wj reduce` tests: a deterministic synthetic multi-file program with an
//! optional planted construct, and helpers to run the reducer through the installed binary.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The planted construct: a distinctive literal inside a small function chain.
pub const PLANT: &str = "7919";

/// `n_modules` modules plus `main.wj`. Every module has a struct, an impl and two functions that
/// call the previous module, so the program is connected and non-trivial. With `plant`, module
/// `plant_at` also contains the planted construct and `main` reaches it.
pub fn synthetic_program(n_modules: usize, plant: Option<usize>) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for i in 0..n_modules {
        let mut s = String::new();
        if i > 0 {
            s.push_str(&format!("use m{}\n\n", i - 1));
        }
        s.push_str(&format!(
            "pub struct Rec{i} {{\n    pub id: int,\n    pub weight: int,\n    pub label: string,\n}}\n\n\
             impl Rec{i} {{\n    pub fn new(id: int) -> Rec{i} {{\n        Rec{i} {{ id: id, weight: id * 3, label: \"rec{i}\" }}\n    }}\n\n\
             \x20   pub fn score(self) -> int {{\n        self.id + self.weight\n    }}\n}}\n\n\
             pub fn make_{i}(n: int) -> int {{\n    let r = Rec{i}::new(n)\n    let mut total = r.score()\n    for k in 0..3 {{\n        total = total + k\n    }}\n    if total > 10 {{\n        total = total - 1\n    }}\n"
        ));
        if i > 0 {
            s.push_str(&format!(
                "    total = total + m{}::make_{}(n)\n",
                i - 1,
                i - 1
            ));
        }
        s.push_str("    total\n}\n\n");
        s.push_str(&format!("pub fn describe_{i}(n: int) -> string {{\n    let r = Rec{i}::new(n)\n    \"${{r.label}}:${{r.score()}}\"\n}}\n"));
        if plant == Some(i) {
            s.push_str(&format!(
                "\npub fn planted_{i}(x: int) -> int {{\n    let y = x * {PLANT}\n    y + 1\n}}\n"
            ));
        }
        files.push((format!("m{i}.wj"), s));
    }
    let mut main = String::new();
    for i in 0..n_modules {
        main.push_str(&format!("use m{i}\n"));
    }
    main.push_str("\nfn main() {\n");
    for i in 0..n_modules {
        main.push_str(&format!(
            "    println(\"${{m{i}::make_{i}({i})}}\")\n    println(m{i}::describe_{i}({i}))\n"
        ));
    }
    if let Some(p) = plant {
        main.push_str(&format!("    println(\"${{m{p}::planted_{p}(2)}}\")\n"));
    }
    main.push_str("}\n");
    files.push(("main.wj".to_string(), main));
    files
}

pub fn write_program(dir: &Path, files: &[(String, String)]) {
    for (rel, text) in files {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
}

pub fn wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

pub fn reduce(input: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(wj())
        .arg("reduce")
        .arg(input)
        .arg("-o")
        .arg(out)
        .args(extra)
        // cargo artifacts of `rustc-error` oracles stay in the test's own scratch space
        .env("WJ_CARGO_TARGET_ROOT", out.with_file_name("cargo-cache"))
        .output()
        .expect("run wj reduce")
}

pub fn read_tree(dir: &Path) -> Vec<(String, String)> {
    let mut v = Vec::new();
    fn walk(base: &Path, dir: &Path, v: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(base, &p, v);
            } else {
                let rel = p
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                v.push((rel, std::fs::read_to_string(&p).unwrap_or_default()));
            }
        }
    }
    walk(dir, dir, &mut v);
    v.sort();
    v
}
