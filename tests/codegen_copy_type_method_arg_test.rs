/// Test that Copy types are passed by value to methods, not by reference
///
/// Bug: Transpiler was adding & to Copy type arguments in method calls
/// This caused type mismatches: expected Entity, found &Entity
#[test]
fn test_copy_type_passed_by_value_to_methods() {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    let source = r#"
@derive(Copy, Clone, Debug, PartialEq, Eq, Hash)
pub struct Entity {
    pub index: i64,
    pub generation: i64,
}

pub struct ComponentArray<T> {
    data: Vec<T>,
}

impl<T> ComponentArray<T> {
    pub fn remove(self, entity: Entity) -> Option<T> {
        None
    }
    
    pub fn get(self, entity: Entity) -> Option<&T> {
        None
    }
}

pub struct World {
    transforms: ComponentArray<int>,
}

impl World {
    pub fn remove_transform(self, entity: Entity) -> Option<int> {
        return self.transforms.remove(entity)
    }
    
    pub fn get_transform(self, entity: Entity) -> Option<&int> {
        return self.transforms.get(entity)
    }
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let result = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should NOT add & to Copy type arguments
    assert!(
        !result.contains(".remove(&entity)"),
        "Should not add & to Copy type in remove()"
    );
    assert!(
        !result.contains(".get(&entity)"),
        "Should not add & to Copy type in get()"
    );
    assert!(
        result.contains(".remove(entity)"),
        "Should pass Copy type by value to remove()"
    );
    assert!(
        result.contains(".get(entity)"),
        "Should pass Copy type by value to get()"
    );
}

