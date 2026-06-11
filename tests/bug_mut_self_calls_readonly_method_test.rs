#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD: Mutating method calling read-only method on self (non-Copy struct)
///
/// Bug: When `update(self)` (inferred &mut self) calls `is_shaking(self)`,
/// the compiler generates `is_shaking(self)` as owned instead of `&self`.
/// This causes E0382 (use of moved value) and E0507 (cannot move out of
/// &mut self).
///
/// The struct MUST be non-Copy (include a `string` field) to reproduce this,
/// because Copy types intentionally use `self` (trivial copy, no moves).
///
/// Expected: `is_shaking` should be inferred as `&self` because it only reads
/// fields. Then `update(&mut self)` can call `self.is_shaking()` without moving.
///
/// Dogfooding source: camera/camera2d.wj
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_readonly_method_gets_ref_self_when_called_from_mut() {
    let source = r#"
pub struct Camera {
    pub shake_timer: f32,
    pub shake_duration: f32,
    pub offset_x: f32,
    pub name: string,
}

impl Camera {
    pub fn is_shaking(self) -> bool {
        self.shake_timer < self.shake_duration
    }

    pub fn update(self, delta: f32) {
        if self.is_shaking() {
            self.shake_timer = self.shake_timer + delta
            if self.shake_timer >= self.shake_duration {
                self.offset_x = 0.0
            }
        }
    }
}

pub fn main() {
    let mut cam = Camera { shake_timer: 0.0, shake_duration: 1.0, offset_x: 5.0, name: "main_camera" }
    cam.update(0.1)
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    assert!(
        rust.contains("fn is_shaking(&self)"),
        "is_shaking should be inferred as &self (read-only on non-Copy struct).\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains("fn update(&mut self"),
        "update should be inferred as &mut self (mutates fields).\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_readonly_chain_all_get_ref_self() {
    let source = r#"
pub struct Timer {
    pub elapsed: f32,
    pub duration: f32,
    pub intensity: f32,
    pub label: string,
}

impl Timer {
    pub fn is_active(self) -> bool {
        self.elapsed < self.duration
    }

    pub fn current_intensity(self) -> f32 {
        if self.is_active() {
            let progress = self.elapsed / self.duration
            self.intensity * (1.0 - progress)
        } else {
            0.0
        }
    }

    pub fn tick(self, delta: f32) {
        self.elapsed = self.elapsed + delta
    }
}

pub fn main() {
    let mut t = Timer { elapsed: 0.0, duration: 1.0, intensity: 5.0, label: "shake" }
    let i = t.current_intensity()
    t.tick(0.1)
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    assert!(
        rust.contains("fn is_active(&self)"),
        "is_active should be &self.\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains("fn current_intensity(&self)"),
        "current_intensity should be &self (only reads + calls &self methods).\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains("fn tick(&mut self"),
        "tick should be &mut self.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_copy_struct_readonly_methods_use_self() {
    let source = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn length(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
}

pub fn main() {
    let v = Vec2 { x: 3.0, y: 4.0 }
    let l = v.length()
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // Copy types should use `self` (by value) — trivially cheap, no moves
    assert!(
        rust.contains("fn length(self)") || rust.contains("fn length(&self)"),
        "Copy type method should use self or &self.\nGenerated:\n{}",
        rust
    );
}
