/// Test: Trait Implementation Self Mutation
///
/// When a trait method declares `fn method(self, ...)` (no & or &mut),
/// but the implementation mutates self, the compiler should infer `&mut self`
/// for the implementation.
///
/// This is a critical feature for game engines where traits define interfaces
/// but implementations need to mutate state.
use std::path::PathBuf;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, code).unwrap();

    let output_dir = temp_dir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let compiler_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wj_binary = compiler_path.join("target/release/wj");

    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = output_dir.join("test.rs");
    let generated_code =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated code");

    Ok(generated_code)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_mutates_self() {
    let code = r#"
        trait GameLoop {
            fn update(self, delta: f32) {
                // Default: do nothing
            }
        }
        
        struct Game {
            frame_count: int,
        }
        
        impl GameLoop for Game {
            fn update(self, delta: f32) {
                self.frame_count = self.frame_count + 1
            }
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // The trait should declare `fn update(&self, delta: f32)` (read-only default)
    // But the impl should use `fn update(&mut self, delta: f32)` (mutates self)
    assert!(
        generated.contains("fn update(&mut self, delta: f32)"),
        "Implementation should use &mut self when it mutates self, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_reads_self() {
    let code = r#"
        trait GameLoop {
            fn render(self) {
                // Default: do nothing
            }
        }
        
        struct Game {
            frame_count: int,
        }
        
        impl GameLoop for Game {
            fn render(self) {
                println!("Frame: {}", self.frame_count)
            }
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // Both trait and impl should use `&self` (read-only)
    assert!(
        generated.contains("fn render(&self)"),
        "Implementation should use &self when it only reads self, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_consumes_self() {
    let code = r#"
        trait GameLoop {
            fn cleanup(self) {
                // Default: do nothing
            }
        }
        
        struct Game {
            name: string,
        }
        
        impl GameLoop for Game {
            fn cleanup(self) {
                println!("Cleanup: {}", self.name)
            }
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // Both should use `&self` for Copy types (string becomes &str for read-only)
    // Or `self` if truly consuming
    assert!(
        generated.contains("fn cleanup(&self)") || generated.contains("fn cleanup(self)"),
        "Implementation should use &self or self for read-only access, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_default_impl_mutates() {
    let code = r#"
        struct Counter {
            count: int,
        }
        
        trait Incrementable {
            fn increment(self) {
                // This would mutate if we had access to self
                // But trait methods can't access self fields directly
            }
        }
        
        impl Incrementable for Counter {
            fn increment(self) {
                self.count = self.count + 1
            }
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // The impl should use `&mut self` because it mutates
    assert!(
        generated.contains("fn increment(&mut self)"),
        "Implementation should use &mut self when it mutates self, got:\n{}",
        generated
    );
}

