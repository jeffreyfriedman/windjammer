//! Compiling is a pure function of (sources, config): repeated builds in one process,
//! different file-creation orders and build order of independent projects must produce
//! byte-identical generated Rust.

use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;
use windjammer::compiler_config::CompilerConfig;
use windjammer::{build_project_with_config, CompilationTarget};

const GEOMETRY: &str = r#"
pub struct Point {
    pub x: f32,
    pub y: f32
}

impl Point {
    pub fn shift(self, dx: f32) {
        self.x = self.x + dx
    }

    pub fn label(self) -> string {
        "point"
    }
}
"#;

const TEXT: &str = r#"
pub fn shout(s: string) -> string {
    s.to_uppercase()
}

pub fn count_chars(s: string) -> i32 {
    s.len() as i32
}
"#;

const STORE: &str = r#"
pub struct Store {
    pub names: Vec<string>
}

impl Store {
    pub fn add(self, name: string) {
        self.names.push(name)
    }

    pub fn total(self) -> i32 {
        self.names.len() as i32
    }
}
"#;

const SOLO: &str = r#"
pub struct Counter {
    pub n: i32
}

impl Counter {
    pub fn bump(self) {
        self.n = self.n + 1
    }
}

pub fn double(x: i32) -> i32 {
    x * 2
}
"#;

type Files<'a> = &'a [(&'a str, &'a str)];

fn collect_rs(dir: &Path, base: &Path, out: &mut BTreeMap<String, String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            collect_rs(&p, base, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
            out.insert(rel, std::fs::read_to_string(&p).unwrap());
        }
    }
}

/// Write `files` in the given order into a fresh project and build it as a library.
fn build(files: Files<'_>) -> BTreeMap<String, String> {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let out = temp.path().join("build");
    for (rel, text) in files {
        let path = src.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    build_project_with_config(
        &src,
        &out,
        CompilationTarget::Rust,
        false,
        true,
        &[],
        &CompilerConfig::default(),
    )
    .expect("fixture project must compile");
    let mut generated = BTreeMap::new();
    collect_rs(&out, &out, &mut generated);
    assert!(!generated.is_empty(), "no Rust output produced");
    generated
}

const PROJECT_A: Files<'static> = &[
    ("shapes/geometry.wj", GEOMETRY),
    ("text/strings.wj", TEXT),
    ("data/store.wj", STORE),
];
const PROJECT_A_REVERSED: Files<'static> = &[
    ("data/store.wj", STORE),
    ("text/strings.wj", TEXT),
    ("shapes/geometry.wj", GEOMETRY),
];
const PROJECT_B: Files<'static> = &[("core/counter.wj", SOLO), ("text/strings.wj", TEXT)];

#[test]
fn same_project_twice_in_one_process_is_byte_identical() {
    assert_eq!(build(PROJECT_A), build(PROJECT_A));
}

#[test]
fn file_creation_order_does_not_change_output() {
    assert_eq!(build(PROJECT_A), build(PROJECT_A_REVERSED));
}

#[test]
fn independent_projects_do_not_influence_each_other() {
    let a_first = build(PROJECT_A);
    let b_second = build(PROJECT_B);
    let b_first = build(PROJECT_B);
    let a_second = build(PROJECT_A);
    assert_eq!(a_first, a_second, "project A output depends on build order");
    assert_eq!(b_first, b_second, "project B output depends on build order");
}
