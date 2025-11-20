use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = PathBuf::from(&crate_dir).join("include");

    // Create output directory
    std::fs::create_dir_all(&output_dir).unwrap();

    // Generate C header
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_language(cbindgen::Language::C)
        .with_pragma_once(true)
        .with_include_guard("WINDJAMMER_H")
        .with_documentation(true)
        .with_cpp_compat(true)
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file(output_dir.join("windjammer.h"));

    println!("cargo:rerun-if-changed=src/lib.rs");
}

