use std::io::Write;
/// TDD Test: No .clone() on Copy fields accessed through iterator loop vars
///
/// Bug: When iterating over a Vec<Struct> and accessing a Copy-type field like
/// `brick.alive` (bool) or `brick.x` (f32), the compiler may add .clone()
/// because it can't infer the loop variable's struct type (and thus can't
/// determine the field is Copy).
///
/// Root Cause: infer_expression_type returns None for iter/iter_mut() calls,
/// so the loop variable's type isn't registered in local_var_types, and field
/// type lookup fails.
///
/// Expected: Copy-type fields should NOT have .clone().
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_no_clone_on_bool_field_via_iter_mut() {
    // Access a bool field through iter_mut() — the exact pattern from brick_breaker.wj
    let source = r#"
pub struct Brick {
    pub x: f32,
    pub y: f32,
    pub alive: bool,
}

pub struct Game {
    pub bricks: Vec<Brick>,
    pub score: i32,
}

impl Game {
    pub fn check_bricks(self) {
        for brick in self.bricks.iter_mut() {
            if !brick.alive {
                continue
            }
            brick.alive = false
            self.score = self.score + 10
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // brick.alive is bool (Copy) — should NOT be cloned
    assert!(
        !generated.contains("brick.alive.clone()"),
        "Should not clone bool field 'alive' accessed through iterator.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_f32_fields_via_loop() {
    // Access f32 fields through a for-loop variable
    let source = r#"
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub speed: f32,
}

pub struct Renderer {
    pub particles: Vec<Particle>,
}

impl Renderer {
    pub fn draw(self) {
        for p in self.particles {
            let px = p.x
            let py = p.y
            let s = p.speed
        }
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // p.x, p.y, p.speed are f32 (Copy) — should NOT be cloned
    assert!(
        !generated.contains("p.x.clone()"),
        "Should not clone f32 field 'x'.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("p.y.clone()"),
        "Should not clone f32 field 'y'.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("p.speed.clone()"),
        "Should not clone f32 field 'speed'.\nGenerated:\n{}",
        generated
    );
}
