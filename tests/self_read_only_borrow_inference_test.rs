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

#[path = "common/test_utils.rs"]
mod test_utils;
#[allow(unused_imports)]
use test_utils::compile_single;

fn compile(input: &str) -> String {
    compile_single(input)
}

/// BUG: Methods that only read self fields are generated with `self` (owned) instead of `&self`.
///
/// In Windjammer, `fn is_shaking(self) -> bool { self.shake_timer < self.shake_duration }`
/// should infer `&self` since the method only reads fields.
///
/// Root cause: The analyzer incorrectly infers OwnershipMode::Owned for methods that
/// only perform comparisons on self fields (when the struct is NOT Copy).
#[test]
fn test_read_only_self_method_gets_borrow() {
    let input = r#"
struct Timer {
    elapsed: f32,
    duration: f32,
    label: String,
}

impl Timer {
    pub fn new(duration: f32) -> Timer {
        Timer { elapsed: 0.0, duration: duration, label: "timer".to_string() }
    }

    pub fn is_done(self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn progress(self) -> f32 {
        self.elapsed / self.duration
    }
}
"#;
    let output = compile(input);
    // is_done only reads fields → should be &self
    assert!(
        output.contains("fn is_done(&self)"),
        "is_done should use &self since it only reads fields.\nGenerated:\n{}",
        output
    );
    // progress only reads fields → should be &self
    assert!(
        output.contains("fn progress(&self)"),
        "progress should use &self since it only reads fields.\nGenerated:\n{}",
        output
    );
}

/// BUG: When a method calls another read-only self method, it should still get &self.
///
/// `shake_intensity` calls `self.is_shaking()` which is also read-only.
/// The analyzer thinks calling a method "on self" means consuming self.
#[test]
fn test_method_calling_read_only_self_method_gets_borrow() {
    let input = r#"
struct Camera {
    shake_timer: f32,
    shake_duration: f32,
    shake_amount: f32,
    name: String,
}

impl Camera {
    pub fn new() -> Camera {
        Camera { shake_timer: 0.0, shake_duration: 0.0, shake_amount: 0.0, name: "cam".to_string() }
    }

    pub fn is_shaking(self) -> bool {
        self.shake_timer < self.shake_duration
    }

    pub fn shake_intensity(self) -> f32 {
        if self.is_shaking() {
            let progress = self.shake_timer / self.shake_duration
            self.shake_amount * (1.0 - progress)
        } else {
            0.0
        }
    }
}
"#;
    let output = compile(input);
    // is_shaking only reads fields → &self
    assert!(
        output.contains("fn is_shaking(&self)"),
        "is_shaking should use &self since it only reads fields.\nGenerated:\n{}",
        output
    );
    // shake_intensity only reads fields and calls a &self method → &self
    assert!(
        output.contains("fn shake_intensity(&self)"),
        "shake_intensity should use &self since it only reads fields and calls a read-only method.\nGenerated:\n{}",
        output
    );
}

/// Methods that modify fields should correctly get &mut self.
#[test]
fn test_mutating_method_gets_mut_borrow() {
    let input = r#"
struct Timer {
    elapsed: f32,
    duration: f32,
    name: String,
}

impl Timer {
    pub fn new(duration: f32) -> Timer {
        Timer { elapsed: 0.0, duration: duration, name: "timer".to_string() }
    }

    pub fn is_done(self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn tick(self, delta: f32) {
        self.elapsed = self.elapsed + delta
    }
}
"#;
    let output = compile(input);
    // tick modifies fields → &mut self
    assert!(
        output.contains("fn tick(&mut self"),
        "tick should use &mut self since it modifies fields.\nGenerated:\n{}",
        output
    );
    // is_done only reads → &self
    assert!(
        output.contains("fn is_done(&self)"),
        "is_done should use &self since it only reads.\nGenerated:\n{}",
        output
    );
}

/// A method with &mut self that calls a read-only method should work.
/// Previously, `self.is_shaking()` inside `update(&mut self)` caused E0507
/// because is_shaking was generated as `self` (owned), so calling it on &mut self
/// would try to move.
#[test]
fn test_mut_method_calling_read_only_no_move() {
    let input = r#"
struct Shaker {
    timer: f32,
    duration: f32,
    offset: f32,
    label: String,
}

impl Shaker {
    pub fn new() -> Shaker {
        Shaker { timer: 0.0, duration: 0.0, offset: 0.0, label: "shaker".to_string() }
    }

    pub fn is_active(self) -> bool {
        self.timer < self.duration
    }

    pub fn update(self, dt: f32) {
        if self.is_active() {
            self.timer = self.timer + dt
            self.offset = self.timer * 2.0
        }
    }
}
"#;
    let output = compile(input);
    // is_active must be &self so it can be called from &mut self context
    assert!(
        output.contains("fn is_active(&self)"),
        "is_active should use &self.\nGenerated:\n{}",
        output
    );
    // update modifies fields → &mut self
    assert!(
        output.contains("fn update(&mut self"),
        "update should use &mut self.\nGenerated:\n{}",
        output
    );
}
