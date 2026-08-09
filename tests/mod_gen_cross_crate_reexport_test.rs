#![cfg(any(not(any(feature = "parser_tests", feature = "analyzer_tests", feature = "codegen_tests", feature = "interpreter_tests", feature = "conformance_tests", feature = "integration_tests")), feature = "integration_tests"))]

#[test]
fn mod_rs_regeneration_preserves_cross_crate_pub_use_without_mod_items() {
    use std::path::Path;
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../windjammerdb/crates/wdb-layers/gen/observability");
    if !out.join("mod.rs").exists() {
        eprintln!("skip: wdb-layers gen not built");
        return;
    }
    let gen_root = out.parent().unwrap();
    let src_root = gen_root.parent().unwrap().join("src");
    std::fs::remove_file(out.join("_mod_items.rs")).ok();
    windjammer::build_utils::generate_mod_file_with_layout(out, Some((gen_root, &src_root))).unwrap();
    let content = std::fs::read_to_string(out.join("mod.rs")).unwrap();
    assert!(
        content.contains("wdb_index::Bm25Index"),
        "cross-crate pub use must survive mod.rs regen without _mod_items.rs\n{}",
        content
    );
}
