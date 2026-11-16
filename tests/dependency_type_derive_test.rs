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

//! Derive inference over a dependency's type: a struct may derive `Serialize` only when every field type implements
//! it. A dependency's type implements it exactly when its crate derives it (recorded in its `metadata.json` as
//! `serde_types`, or read from its generated Rust), whatever its fields are. `meter::Bucket` (two ints) is generated
//! without `Serialize`, so the app's `Hub { buckets: Vec<Bucket> }` must not derive it (E0277), while `Note` (text
//! only) still does. (Compiling the app needs serde and the runtime crate, so the check reads the derives.)

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn wj(args: &[&str], cwd: &Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(cwd).output().unwrap();
    assert!(out.status.success(), "wj {args:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn struct_holding_a_dependency_type_without_serialize_does_not_derive_it() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    write(t, "meter/wj.toml", "[package]\nname = \"meter\"\nversion = \"0.1.0\"\n");
    write(t, "meter/src/lib.wj", "pub struct Bucket {\n    pub start_ms: int,\n    pub count: int,\n}\n");
    write(t, "app/wj.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmeter = { path = \"../meter_gen\" }\n");
    write(
        t,
        "app/src/main.wj",
        "use std::json\nuse meter::Bucket\n\npub struct Note {\n    pub text: string,\n}\n\npub struct Hub {\n    pub name: string,\n    pub buckets: Vec<Bucket>,\n}\n\n\
         fn main() {\n    let hub = Hub { name: \"h\", buckets: [Bucket { start_ms: 0, count: 0 }] }\n    match json.to_string(Note { text: hub.name }) {\n        Ok(t) => println(\"${t} ${hub.buckets.len()}\"),\n        Err(_) => println(\"err\"),\n    }\n}\n",
    );
    wj(&["build", "src", "--library", "--no-cargo", "-o", "../meter_gen"], &t.join("meter"));
    let meta = fs::read_to_string(t.join("meter_gen/metadata.json")).unwrap();
    assert!(meta.contains("\"serde_types\": []"), "the dependency records that it derives Serialize for nothing:\n{meta}");
    wj(&["build", "src", "--no-cargo", "-o", "../app_gen"], &t.join("app"));
    let main_rs = fs::read_to_string(t.join("app_gen/main.rs")).unwrap();
    let derive_of = |item: &str| {
        let at = main_rs.find(item).unwrap_or_else(|| panic!("{item} missing:\n{main_rs}"));
        main_rs[..at].rfind("#[derive(").map(|d| main_rs[d..at].to_string()).unwrap_or_default()
    };
    assert!(!derive_of("pub struct Hub").contains("Serialize"), "Hub holds meter::Bucket (no Serialize):\n{main_rs}");
    assert!(derive_of("pub struct Note").contains("Serialize"), "Note is serialized:\n{main_rs}");
}
