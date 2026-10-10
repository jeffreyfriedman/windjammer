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
))]

//! `for i in 0..len()` must stay `usize` when the return is `i as i32`.
//!
//! `assets/pipeline.wj` `find_texture` writes `return i as i32` inside
//! `for i in 0..self.textures.len()` and indexes `self.textures[i]`.
//! `self.textures[(i as usize)]`. The source cast says `i` is not already `i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i32_range(body: &str) -> bool {
    body.contains("0_i32..") || body.contains("len() as i32") || body.contains("i as usize")
}

#[test]
fn index_loop_returned_as_i32_must_stay_usize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Tex {
    pub name: string,
}

pub fn find_texture(textures: Vec<Tex>, name: string) -> i32 {
    for i in 0..textures.len() {
        if textures[i].name == name {
            return i as i32
        }
    }
    -1
}
"#,
    );
    let map = test
        .compile()
        .expect("index loop returned as i32 must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i32_range(body),
        "for i in 0..len() must stay usize when the source returns i as i32; got:\n{body}"
    );
}

