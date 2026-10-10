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

//! Copy `Vec3` into **method** owned formal must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub fn rotate_vec3(self, vec: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z)
        let uv = qv.cross(vec)
        let uuv = qv.cross(uv)
        uv
    }
}

pub fn look_at_helper(eye: Vec3, f: Vec3, s: Vec3) -> f32 {
    let u = s.cross(f)
    s.dot(eye) + u.x
}
"#;

#[test]
fn module_file_copy_vec3_method_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("vec.clone()")
        || rs.contains("uv.clone()")
        || rs.contains("f.clone()")
        || rs.contains("eye.clone()");
    assert!(
        !bad,
        "RED Copy Vec3 cloned into method owned formal:\n{rs}"
    );
    let _ = test.cargo_check();
}
