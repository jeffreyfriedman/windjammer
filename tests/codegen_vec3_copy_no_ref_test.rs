use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_vec3_copy_method_no_ref() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_vec3_copy_no_ref");
    
    fs::create_dir_all(&test_dir).unwrap();

    // Test that Vec3 (Copy type) methods don't add & to arguments
    // Vec3.add(v: Vec3) takes Vec3 by value (Copy), not &Vec3
    let test_content = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
    
    fn mul(self, scalar: f32) -> Vec3 {
        Vec3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

struct Particle {
    position: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
}

impl Particle {
    fn update(&mut self, dt: f32) {
        self.velocity = self.velocity.add(self.acceleration.mul(dt));
        self.position = self.position.add(self.velocity.mul(dt));
    }
}

fn main() {
    let mut particle = Particle {
        position: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        velocity: Vec3 { x: 1.0, y: 2.0, z: 3.0 },
        acceleration: Vec3 { x: 0.0, y: -9.8, z: 0.0 },
    };
    particle.update(0.1);
    println!("Position: {}, {}, {}", particle.position.x, particle.position.y, particle.position.z);
}
"#;

    let test_file = test_dir.join("vec3_copy_no_ref.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join("vec3_copy_no_ref.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should NOT add & for Copy type method call results
    // self.velocity.add(self.acceleration.mul(dt)) NOT self.velocity.add(&self.acceleration.mul(dt))
    assert!(
        rust_code.contains("self.velocity.add(self.acceleration.mul(dt))"),
        "Expected NO auto-ref for Copy type method call result.\nGenerated code:\n{}",
        rust_code
    );

    // Should NOT contain the incorrect version with &
    assert!(
        !rust_code.contains("self.velocity.add(&self.acceleration"),
        "Should NOT add & to method call results for Copy types.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("vec3_copy_no_ref.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

