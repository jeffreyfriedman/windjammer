#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Dogfooding: ecs/world.wj `serialize_vec3(v: Vec3)` reads Copy fields via `v.x.to_le_bytes()`.
/// Unknown stdlib methods on Copy fields must NOT infer `&mut` on the parent parameter.
#[test]
fn test_copy_field_to_le_bytes_param_stays_owned() {
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Transform {
    position: Vec3,
    rotation: Vec3,
    scale: Vec3,
}

fn serialize_vec3(v: Vec3) -> Vec<u8> {
    let mut bytes = Vec::new()
    let x_bits = v.x.to_le_bytes()
    bytes.push(x_bits[0])
    bytes
}

fn serialize_transform(t: Transform) -> Vec<u8> {
    let pos = t.position
    let rot = t.rotation
    let scale = t.scale
    let mut bytes = serialize_vec3(pos)
    let rot_bytes = serialize_vec3(rot)
    let scale_bytes = serialize_vec3(scale)
    let mut i = 0
    while i < rot_bytes.len() {
        bytes.push(rot_bytes[i])
        i = i + 1
    }
    i = 0
    while i < scale_bytes.len() {
        bytes.push(scale_bytes[i])
        i = i + 1
    }
    bytes
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        !generated.contains("v: &mut Vec3"),
        "serialize_vec3 param must be owned Vec3, not &mut. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("v: Vec3"),
        "serialize_vec3 should take owned Vec3. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("serialize_vec3(&mut pos)"),
        "call sites must pass owned Copy locals, not &mut. Generated:\n{}",
        generated
    );
}
