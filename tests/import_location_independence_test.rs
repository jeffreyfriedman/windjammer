#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! Differential gate: the `use` lines of a generated crate depend on the project's declarations only, never on what
//! the output directory is called or where the checkout lives. One project (nested directory modules, a sibling
//! import, a crate import, a root module importing a directory module) is built into output directories named `build`, `out`,
//! `generated` and `gen_x`, from checkouts at different depths, with and without a sibling directory named like a
//! module; every build must produce the same `use` lines, and one of them must compile.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn project(root: &Path) {
    write(root, "app/wj.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"app\"\n");
    write(root, "app/src/lib.wj", "pub mod shapes\npub mod scene\npub mod top\n");
    write(root, "app/src/top.wj", "use shapes::circle::Circle\n\npub fn unit() -> Circle {\n    Circle { r: 1 }\n}\n");
    write(root, "app/src/shapes/mod.wj", "pub mod circle\npub mod square\n");
    write(root, "app/src/shapes/circle.wj", "pub struct Circle {\n    pub r: int,\n}\n");
    write(
        root,
        "app/src/shapes/square.wj",
        "use circle::Circle\n\npub struct Square {\n    pub side: int,\n}\n\npub fn inscribed(s: Square) -> Circle {\n    Circle { r: s.side / 2 }\n}\n",
    );
    write(
        root,
        "app/src/scene/mod.wj",
        "pub mod stage\n",
    );
    write(
        root,
        "app/src/scene/stage.wj",
        "use crate::shapes::circle::Circle\nuse shapes::square::Square\n\npub fn area(c: Circle, s: Square) -> int {\n    c.r * c.r * 3 + s.side * s.side\n}\n",
    );
}

/// `file -> use lines` of every generated `.rs` under `out`.
fn use_lines(out: &Path) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    let mut stack = vec![out.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = fs::read_to_string(&p).unwrap();
                let uses = text.lines().filter(|l| l.trim_start().starts_with("use ") || l.trim_start().starts_with("pub use ")).map(str::to_string).collect();
                map.insert(p.strip_prefix(out).unwrap().display().to_string(), uses);
            }
        }
    }
    map
}

fn build(checkout: &Path, out_name: &str) -> (BTreeMap<String, Vec<String>>, std::path::PathBuf) {
    project(checkout);
    let out = checkout.join(out_name);
    let status = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "src", "--library", "--module-file", "--no-cargo", "-o"])
        .arg(&out)
        .current_dir(checkout.join("app"))
        .output()
        .unwrap();
    assert!(status.status.success(), "wj build into {out_name} failed:\n{}", String::from_utf8_lossy(&status.stderr));
    (use_lines(&out), out)
}

#[test]
fn use_lines_do_not_depend_on_output_name_or_checkout_location() {
    let tmp = tempfile::tempdir().unwrap();
    let mut results = Vec::new();
    for (checkout, out_name) in [("a", "build"), ("b/deeper/checkout", "out"), ("c", "generated"), ("d/x", "gen_x")] {
        let root = tmp.path().join(checkout);
        results.push((format!("{checkout}/{out_name}"), build(&root, out_name)));
    }
    // A directory beside the output named like a module of the crate must not change anything.
    let e = tmp.path().join("e");
    fs::create_dir_all(e.join("shapes")).unwrap();
    results.push(("e/out_beside_shapes".to_string(), build(&e, "out_beside_shapes")));

    let (first_name, (first, first_out)) = &results[0];
    assert!(!first.is_empty());
    for (name, (uses, _)) in &results[1..] {
        assert_eq!(uses, first, "`use` lines of {name} differ from {first_name}");
    }

    // rustc accepts the crate (independent oracle).
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings", "--crate-type", "lib", "--crate-name", "app", "-o"])
        .arg(tmp.path().join("libapp.rlib"))
        .arg(first_out.join("lib.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "rustc rejected the crate:\n{}\n{first:#?}", String::from_utf8_lossy(&rustc.stderr));
}
