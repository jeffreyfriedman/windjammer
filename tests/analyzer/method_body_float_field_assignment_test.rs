use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_method_body_field_assignment_infers_f32() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("timer.wj"),
        r#"
pub struct Timer {
    pub elapsed: f32
}

impl Timer {
    pub fn reset(self) {
        self.elapsed = 0.0
    }

    pub fn tick(self, delta: f32) {
        self.elapsed = self.elapsed + delta
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Should compile");

    let timer_code = std::fs::read_to_string(build.join("timer.rs")).unwrap();

    assert!(
        timer_code.contains("self.elapsed = 0.0_f32"),
        "Expected 'self.elapsed = 0.0_f32' in reset(). Generated:\n{}",
        timer_code
    );

    assert!(
        !timer_code.contains("self.elapsed = 0.0_f64"),
        "Should not generate f64 when field is f32"
    );
}

#[test]
fn test_state_machine_time_in_state() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("machine.wj"),
        r#"
pub struct StateMachine {
    pub current_state: i32,
    pub time_in_state: f32
}

impl StateMachine {
    pub fn transition(self, new_state: i32) -> bool {
        if self.current_state == new_state {
            false
        } else {
            self.current_state = new_state
            self.time_in_state = 0.0
            true
        }
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Should compile");

    let machine_code = std::fs::read_to_string(build.join("machine.rs")).unwrap();

    assert!(
        machine_code.contains("self.time_in_state = 0.0_f32"),
        "Expected 'self.time_in_state = 0.0_f32' in transition(). Generated:\n{}",
        machine_code
    );
}

/// Library multipass: nested path under src/ forces `build_library_multipass` (game-style layout).
#[test]
fn test_nested_module_method_body_field_assignment_infers_f32() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("ai")).unwrap();

    std::fs::write(
        src.join("ai/state_machine.wj"),
        r#"
pub struct StateMachine {
    pub current_state: i32,
    pub time_in_state: f32
}

impl StateMachine {
    pub fn transition(self, new_state: i32) -> bool {
        if self.current_state == new_state {
            false
        } else {
            self.current_state = new_state
            self.time_in_state = 0.0
            true
        }
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Should compile");

    let path = build.join("ai/state_machine.rs");
    assert!(path.exists(), "expected output at {:?}", path);
    let code = std::fs::read_to_string(&path).unwrap();

    assert!(
        code.contains("self.time_in_state = 0.0_f32"),
        "Expected 'self.time_in_state = 0.0_f32' in multipass nested module. Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.time_in_state = 0.0_f64"),
        "Should not generate f64 for f32 field in multipass build.\n{}",
        code
    );
}
