#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! A crate build (every source of the crate compiled, the library multipass build) never reads the crate's own
//! project-root `metadata.json` back as a signature source: it is an earlier build's output (or a stale file
//! committed with the sources). Merged last as "authoritative", a stale `area(&self, origin: &Point, ..)` from an
//! older compiler became the call-site contract while the definition was emitted from this build's analysis
//! (`origin: Point`), and callers borrowed a by-value formal (rustc E0308).

use std::fs;
use tempfile::TempDir;

const STALE_ROOT_METADATA: &str = r#"{"version":"0.1.0","structs":{},"copy_structs":[],"string_consts":[],
"functions":{"Probe::area":{"params":["Custom(\"Self\")","Reference(Custom(\"Point\"))"],
"formal_params":["Custom(\"Self\")","Custom(\"Point\")"],"return_type":"Bool","is_associated":true,
"parent_type":"Probe","param_ownership":["Borrowed","Borrowed"],"emitted_rust_ref_params":[false,true],
"string_ref_string_formal_params":null,"forwarding_borrow_params":[false,false],"has_self_receiver":true,
"is_extern":false,"is_trait_method":false}}}"#;

fn project_with_stale_root_metadata() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().join("crate");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n").unwrap();
    fs::write(root.join("src/probe.wj"), "pub struct Probe { pub r: f32 }\n").unwrap();
    fs::write(root.join("metadata.json"), STALE_ROOT_METADATA).unwrap();
    tmp
}

#[test]
fn test_crate_build_does_not_merge_its_own_root_metadata() {
    let tmp = project_with_stale_root_metadata();
    let src = tmp.path().join("crate/src");
    let mut registry = windjammer::analyzer::SignatureRegistry::new();
    windjammer::metadata::merge_wj_meta_signatures_for_crate_build(&src, &tmp.path().join("cache"), &mut registry);
    assert!(
        registry.get_signature("Probe::area").is_none(),
        "the crate's own earlier metadata.json must not feed its build: {:?}",
        registry.get_signature("Probe::area").map(|s| s.param_ownership.clone())
    );
}

/// A build that does not compile the crate (a single file reading its crate's signatures) still reads it.
#[test]
fn test_single_file_build_still_reads_crate_root_metadata() {
    let tmp = project_with_stale_root_metadata();
    let src = tmp.path().join("crate/src");
    let mut registry = windjammer::analyzer::SignatureRegistry::new();
    windjammer::metadata::merge_wj_meta_signatures_from_dir(&src, &tmp.path().join("cache"), &mut registry);
    assert!(registry.get_signature("Probe::area").is_some(), "crate metadata is an input outside a crate build");
}
