/// TDD: Final float comparison/assignment tests for remaining 6 E0277 errors
///
/// Patterns from build_errors.log:
/// - physics_body: self.velocity.x = 0.0, self.velocity.x != 0.0 (assignment + comparison)
/// - quick_start/game: self.camera.position.x != 0.0 (3-level nested)
/// - post_processing: self.settings.gamma != 1.0 (2-level nested)
#[path = "test_utils.rs"]
mod test_utils;

// =============================================================================
// Assignment: self.velocity.x = 0.0 (physics_body pattern)
// =============================================================================

#[test]
fn test_field_assignment_literal() {
    let source = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }
}

pub struct PhysicsBody { pub velocity: Vec3 }

impl PhysicsBody {
    pub fn stop_x(self) {
        self.velocity.x = 0.0
    }
    pub fn stop_z(self) {
        self.velocity.z = 0.0
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.0_f32"),
        "self.velocity.x = 0.0 should generate 0.0_f32 (assignment LHS→RHS), got:\n{}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Assignment to f32 field should NOT use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Two-level nested: self.settings.gamma != 1.0 (post_processing pattern)
// =============================================================================

#[test]
fn test_two_level_nested_comparison() {
    let source = r#"
pub struct ColorGradingSettings { pub gamma: f32 }

impl ColorGradingSettings {
    pub fn new() -> ColorGradingSettings {
        ColorGradingSettings { gamma: 1.0 }
    }
}

pub struct ColorGrading { pub settings: ColorGradingSettings }

impl ColorGrading {
    pub fn needs_gamma_correction(self) -> bool {
        self.settings.gamma != 1.0
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "self.settings.gamma != 1.0 should generate 1.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Two-level nested comparison should NOT use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Three-level nested: self.camera.position.x != 0.0 (quick_start pattern)
// =============================================================================

#[test]
fn test_three_level_nested_comparison() {
    let source = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }
}

pub struct Camera { pub position: Vec3 }

impl Camera {
    pub fn default() -> Camera {
        Camera { position: Vec3::new(0.0, 0.0, 0.0) }
    }
}

pub struct QuickStartGame { pub camera: Camera }

impl QuickStartGame {
    pub fn is_ready(self) -> bool {
        self.camera.position.x != 0.0 || self.camera.position.y != 0.0 || self.camera.position.z != 0.0
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.0_f32"),
        "self.camera.position.x != 0.0 should generate 0.0_f32 (3-level nested), got:\n{}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Three-level nested comparison should NOT use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Binary division: 1.0 / self.settings.gamma (post_processing inv_gamma)
// =============================================================================

#[test]
fn test_binary_rhs_field_propagation() {
    let source = r#"
pub struct ColorGradingSettings { pub gamma: f32 }

impl ColorGradingSettings {
    pub fn new() -> ColorGradingSettings {
        ColorGradingSettings { gamma: 1.0 }
    }
}

pub struct ColorGrading { pub settings: ColorGradingSettings }

impl ColorGrading {
    pub fn compute_inv_gamma(self) -> f32 {
        1.0 / self.settings.gamma
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "1.0 / self.settings.gamma should infer 1.0 as f32 (RHS→LHS), got:\n{}",
        output
    );
}
