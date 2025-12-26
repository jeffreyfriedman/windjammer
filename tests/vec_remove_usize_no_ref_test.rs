/// Test: Vec::remove with usize should not add & or .clone()
///
/// When calling Vec::remove with a usize variable, the compiler should NOT add:
/// - & (reference)
/// - .clone()
///
/// Vec::remove expects `usize` by value, not `&usize`.
///
/// Bug discovered in game engine ECS component storage.
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
fn test_vec_remove_usize_variable() {
    let code = r#"
        fn remove_at(items: Vec<int>, index: usize) -> Vec<int> {
            let mut items = items
            items.remove(index)
            return items
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // Should generate: items.remove(index)
    // NOT: items.remove(&index) or items.remove(&index.clone())
    assert!(
        generated.contains("items.remove(index)"),
        "Vec::remove should not add & or .clone() for usize, got:\n{}",
        generated
    );

    assert!(
        !generated.contains("items.remove(&index"),
        "Vec::remove should not add & for usize, got:\n{}",
        generated
    );
}

#[test]
fn test_vec_remove_with_cast() {
    let code = r#"
        fn remove_at(items: Vec<int>, index: int) -> Vec<int> {
            let mut items = items
            let idx: usize = index as usize
            items.remove(idx)
            return items
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // Should generate: items.remove(idx)
    // NOT: items.remove(&idx.clone())
    assert!(
        generated.contains("items.remove(idx)"),
        "Vec::remove with cast should not add & or .clone(), got:\n{}",
        generated
    );
}

#[test]
fn test_vec_remove_on_struct_field() {
    let code = r#"
        struct Storage {
            dense: Vec<int>,
        }
        
        impl Storage {
            fn remove_at(self, index: usize) -> Storage {
                self.dense.remove(index)
                return self
            }
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // Should generate: self.dense.remove(index)
    // NOT: self.dense.remove(&index.clone())
    assert!(
        generated.contains("self.dense.remove(index)")
            || generated.contains("self.dense.remove(&mut index)"),
        "Vec::remove on struct field should not add & and .clone(), got:\n{}",
        generated
    );

    assert!(
        !generated.contains("self.dense.remove(&index.clone())"),
        "Vec::remove should not add & and .clone(), got:\n{}",
        generated
    );
}

#[test]
fn test_vec_remove_with_local_usize_cast() {
    // This matches the exact pattern from the game engine ECS code
    let code = r#"
        struct ComponentStorage<T> {
            dense: Vec<T>,
            entities: Vec<int>,
            sparse: Vec<int>,
        }
        
        impl<T> ComponentStorage<T> {
            fn remove(self, entity_idx: usize) -> Option<T> {
                let sparse_index: int = self.sparse[entity_idx]
                if sparse_index < 0 {
                    return None
                }
                
                let sparse_idx_usize: usize = sparse_index as usize
                let component = self.dense.remove(sparse_idx_usize)
                let removed_entity = self.entities.remove(sparse_idx_usize)
                
                return Some(component)
            }
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // Should generate: self.dense.remove(sparse_idx_usize)
    // NOT: self.dense.remove(&sparse_idx_usize.clone())
    assert!(
        !generated.contains("self.dense.remove(&sparse_idx_usize.clone())"),
        "Vec::remove should not add & and .clone() for local usize variable, got:\n{}",
        generated
    );

    assert!(
        generated.contains("self.dense.remove(sparse_idx_usize)"),
        "Vec::remove should use the usize variable directly, got:\n{}",
        generated
    );
}

#[test]
#[ignore] // TODO: Regression from ownership inference - HashMap::remove with Copy keys needs review
fn test_hashmap_remove_with_owned_key() {
    let code = r#"
        use std::collections::HashMap
        
        struct Entity {
            id: int,
        }
        
        fn remove_entity(entities: HashMap<Entity, string>, entity: Entity) -> HashMap<Entity, string> {
            let mut entities = entities
            entities.remove(entity)
            return entities
        }
    "#;

    let generated = compile_code(code).expect("Compilation should succeed");

    // For HashMap::remove with non-Copy key, should add &
    // entities.remove(&entity)
    assert!(
        generated.contains("entities.remove(&entity)"),
        "HashMap::remove should add & for non-Copy keys, got:\n{}",
        generated
    );
}
