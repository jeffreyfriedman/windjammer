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

//! P3.368: i32-coord locals/literals must cast into i64 entity APIs and u32 FFI/struct slots.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod ecs
pub mod tex
"#;

const ECS: &str = r#"
pub fn register_entity(entity_id: i64) {
    let _ = entity_id
}

pub fn touch() {
    let entity = 1
    register_entity(entity)
}
"#;

const TEX: &str = r#"
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub handle: u64,
}

extern fn gradient_sprite(width: u32, height: u32) -> u64

pub fn test_gradient() -> Texture {
    let width = 256
    let height = 256
    let handle = gradient_sprite(width, height)
    Texture { width, height, handle: handle }
}
"#;

fn bad_i32_into_i64_or_u32(rs_ecs: &str, rs_tex: &str) -> bool {
    (rs_ecs.contains("register_entity(entity)") && !rs_ecs.contains("as i64"))
        || rs_ecs.contains("register_entity(1_i32")
        || (rs_tex.contains("gradient_sprite(width, height)")
            && !rs_tex.contains("as u32"))
        || rs_tex.contains("Texture { width, height") && rs_tex.contains("width, height, handle") 
            && (rs_tex.matches("width,").next().is_some() && rs_tex.contains("{ width, height") && !rs_tex.contains("width as u32") && rs_tex.contains("let width"))
}

#[test]
fn i32_binding_into_i64_and_u32_formal_must_cast() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ecs.wj", ECS);
    test.add_file("tex.wj", TEX);
    let map = test.compile().expect("P3.368 compile");
    let ecs = map.get("ecs.rs").expect("ecs.rs");
    let tex = map.get("tex.rs").expect("tex.rs");
    if bad_i32_into_i64_or_u32(ecs, tex) {
        eprintln!("P3.368 RED ecs:\n{ecs}\ntex:\n{tex}");
    }
    assert!(
        ecs.contains("as i64") || ecs.contains("1_i64"),
        "P3.368: i32 entity binding must cast to i64:\n{ecs}"
    );
    assert!(
        tex.contains("as u32") || tex.contains("256_u32"),
        "P3.368: i32 width/height must satisfy u32 formals/fields:\n{tex}"
    );
    test.cargo_check().expect("P3.368 cargo-check");
}
