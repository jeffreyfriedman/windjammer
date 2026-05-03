//! TDD: Backward type propagation for float literals in variable initialization
//!
//! Problem: Variables initialized with float literals get wrong type when used later.
//! Example: `let offset_x = 0.0` defaults to f64, but `self.player.position.x + offset_x`
//! needs f32 + f32.
//!
//! Root Cause: Inference runs in single pass, doesn't propagate constraints backward
//! from usage site to initialization.

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_variable_used_with_f32_field() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Player {
    pub position: Vec3,
}

pub struct Camera {
    pub player: Player,
}

impl Camera {
    pub fn update_camera(self) {
        let offset_x = 0.0
        let offset_y = 5.0
        let offset_z = -10.0

        let cam_x = self.player.position.x + offset_x
        let cam_y = self.player.position.y + offset_y
        let cam_z = self.player.position.z + offset_z
    }
}
"#;

    let rust = test_utils::compile_single(source);

    // offset variables should be inferred as f32 (backward propagation from usage)
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "offset_x = 0.0 should generate 0.0_f32 when used with f32 field, got:\n{}",
        rust
    );
    assert!(
        rust.contains("5.0_f32") || rust.contains("5.0f32"),
        "offset_y = 5.0 should generate 5.0_f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("-10.0_f32") || rust.contains("-10.0f32"),
        "offset_z = -10.0 should generate -10.0_f32, got:\n{}",
        rust
    );

    // Verify it compiles
    test_utils::verify_rust_compiles(&rust).expect("Generated Rust should compile");
}
