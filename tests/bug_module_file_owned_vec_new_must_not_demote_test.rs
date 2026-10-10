#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//!
//!   `FABRIKChain::new(joint_positions: &Vec<Vec3>)` then `joints: joint_positions.clone()`
//!   call site `FABRIKChain::new(&joints)`
//!   `VoxParser::new(data: &Vec<u8>)` + `VoxParser::new(&data)`
//! WJ source is `new(joint_positions: Vec<Vec3>)` / `new(data: Vec<u8>)` and
//! `Chain::new(joints)` / `VoxParser::new(data)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Chain {
    pub joints: Vec<i32>,
}

impl Chain {
    pub fn new(joints: Vec<i32>) -> Chain {
        Chain { joints: joints }
    }
}

pub fn make() -> Chain {
    let joints = vec![1, 2, 3]
    Chain::new(joints)
}
"#;

/// field was demoted to `pub fn new(data: &Vec<u8>)` + `.clone()`.
const PRODUCT_VOX: &str = r#"
struct VoxParser {
    data: Vec<u8>,
    pos: int,
}

impl VoxParser {
    fn new(data: Vec<u8>) -> VoxParser {
        VoxParser { data: data, pos: 0 }
    }
}

pub fn parse(data: Vec<u8>) -> int {
    let parser = VoxParser::new(data)
    parser.pos
}
"#;

#[test]
fn module_file_owned_vec_new_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let demoted_formal = rs.contains("fn new(joints: &Vec")
        || rs.contains("fn new(joints: &mut Vec")
        || rs.contains("new(joints: &Vec<i32>)");
    let borrowed_call = rs.contains("Chain::new(&joints)") || rs.contains("new(&joints)");
    let field_clone = rs.contains("joints: joints.clone()")
        || rs.contains("joints: joint_positions.clone()");
    assert!(
        !demoted_formal && !borrowed_call && !field_clone,
        "RED: owned Vec new() demoted to &Vec + clone:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_private_associated_vec_new_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("vox.wj", PRODUCT_VOX);
    let map = test.compile().expect("multi-field compile");
    let rs = map.get("vox.rs").expect("vox.rs");
    eprintln!("multi-field vox.rs:\n{rs}");
    assert!(
        !rs.contains("fn new(data: &Vec") && !rs.contains("VoxParser::new(&data)"),
        "RED: private associated Vec new demoted:\n{rs}"
    );
    assert!(
        !rs.contains("data: data.clone()"),
        "RED: demoted Vec new cloned into field:\n{rs}"
    );
    test.cargo_check().expect("multi-field cargo-check");
}

