//! A library build writes only into its output directory and its cache: the source tree, including
//! hand-written Rust next to the Windjammer sources (`src/lib.rs`, `src/tests/*.rs`), is left byte for byte.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Every file under `root` except the output and cache directories (relative path -> bytes).
fn source_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name == "gen" || name.starts_with(".wj") {
                continue;
            }
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.insert(p.strip_prefix(base).unwrap().display().to_string(), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn library_build_leaves_the_source_tree_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let files = [
        ("src/mod.wj", "pub mod shapes\npub mod util\n"),
        ("src/shapes.wj", "pub struct Square {\n    pub side: f32,\n}\n\nimpl Square {\n    pub fn area(self) -> f32 {\n        self.side * self.side\n    }\n}\n"),
        ("src/util.wj", "use crate::shapes::Square\n\npub fn total(items: Vec<Square>) -> f32 {\n    let mut t = 0.0\n    for s in items {\n        t = t + s.area()\n    }\n    t\n}\n"),
        ("src/lib.rs", "// hand-written crate root\npub mod shapes;\npub mod util;\n"),
        ("src/tests/area_test.rs", "#[test]\nfn area() {\n    assert_eq!(1 + 1, 2);\n}\n"),
        ("Cargo.toml", "[package]\nname = \"shapes\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let before = source_tree(root);
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "src/mod.wj", "--output", "gen", "--library", "--no-cargo", "--module-file"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(root.join("gen").is_dir(), "no output written");
    let after = source_tree(root);
    let changed: Vec<&String> = before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|k| before.get(*k) != after.get(*k))
        .collect();
    assert!(changed.is_empty(), "the build wrote into the source tree: {changed:?}");
}
