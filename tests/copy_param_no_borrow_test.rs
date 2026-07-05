#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let input = dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test.wj");
    let output = dir.path().join("output");
    std::fs::create_dir_all(&output).expect("create output dir");

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", input.to_str().unwrap(), "--no-cargo", "-o"])
        .arg(output.to_str().unwrap())
        .output()
        .expect("run wj");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let generated_path = output.join("test.rs");
    let generated = if generated_path.exists() {
        std::fs::read_to_string(&generated_path).unwrap_or_default()
    } else {
        String::new()
    };

    (result.status.success(), generated, combined)
}

#[test]
fn test_copy_struct_param_not_borrowed_for_owned_callee() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Perception {
    pub detection: f32,
}

impl Perception {
    pub fn is_in_sight_cone(
        npc_pos: Vec3,
        npc_forward: Vec3,
        target_pos: Vec3,
    ) -> bool {
        let dx = target_pos.x - npc_pos.x
        let dz = target_pos.z - npc_pos.z
        let dot = dx * npc_forward.x + dz * npc_forward.z
        dot > 0.0
    }

    pub fn process_sight(
        npc_pos: Vec3,
        npc_forward: Vec3,
        target_pos: Vec3,
    ) {
        let in_sight = self.is_in_sight_cone(npc_pos, npc_forward, target_pos)
        if in_sight {
            self.detection = 1.0
        }
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        !generated.contains("&npc_forward"),
        "Copy param npc_forward should not be borrowed when callee expects owned Vec3. Generated:\n{}",
        generated
    );
}

#[test]
fn test_hashmap_get_with_option_binding_no_deref() {
    let source = r#"
use std::collections::HashMap

pub struct AnimationData {
    pub frame_count: i32,
}

pub struct AnimController {
    pub animations: HashMap<string, AnimationData>,
    pub current_animation: Option<string>,
}

impl AnimController {
    pub fn current_frame(self) -> i32 {
        if let Some(anim_name) = self.current_animation {
            if let Some(animation) = self.animations.get(anim_name) {
                return animation.frame_count
            }
        }
        0
    }
}
"#;
    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    assert!(
        !generated.contains("*anim_name"),
        "Option binding anim_name should not be dereferenced for HashMap::get. Generated:\n{}",
        generated
    );
}
