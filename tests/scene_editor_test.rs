/// TDD Test: Scene Editor - Create and Select Entities
///
/// The scene editor needs to:
/// 1. Create new entities with components
/// 2. Select entities (single and multiple)
/// 3. Delete entities
/// 4. Duplicate entities
/// 5. Track selection state
///
/// This is the foundation for the visual editor UI.
use std::path::PathBuf;
use tempfile::TempDir;

fn compile_and_run_wj(code: &str) -> Result<String, String> {
    use std::fs;
    use std::process::Command;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let output_dir = temp_dir.path().to_path_buf();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).map_err(|e| format!("Failed to write test file: {}", e))?;

    // Compile Windjammer to Rust
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(&output_dir)
        .output()
        .map_err(|e| format!("Failed to execute compiler: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        ));
    }

    // Compile Rust to executable
    let rs_file = output_dir.join("test.rs");
    let exe_file = output_dir.join("test");

    let rustc_result = Command::new("rustc")
        .arg(&rs_file)
        .arg("-o")
        .arg(&exe_file)
        .arg("--edition")
        .arg("2021")
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !rustc_result.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_result.stderr);
        return Err(format!("Rust compilation failed:\n{}", stderr));
    }

    // Run the executable
    let run_result = Command::new(&exe_file)
        .output()
        .map_err(|e| format!("Failed to run executable: {}", e))?;

    let stdout = String::from_utf8_lossy(&run_result.stdout);
    Ok(stdout.to_string())
}

#[test]
fn test_scene_editor_create_entity() {
    let code = r#"
struct Transform {
    pub x: f32,
    pub y: f32,
}

struct EditorState {
    pub next_entity_id: i64,
    pub entities: Vec<i64>,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            next_entity_id: 1,
            entities: Vec::new(),
        }
    }

    pub fn create_entity(&mut self) -> i64 {
        let id = self.next_entity_id
        self.next_entity_id = self.next_entity_id + 1
        self.entities.push(id)
        id
    }

    pub fn entity_count(&self) -> i64 {
        self.entities.len() as i64
    }
}

fn main() {
    let mut editor = EditorState::new()
    
    let entity1 = editor.create_entity()
    let entity2 = editor.create_entity()
    let entity3 = editor.create_entity()
    
    println!("Created entity: {}", entity1)
    println!("Created entity: {}", entity2)
    println!("Created entity: {}", entity3)
    println!("Total entities: {}", editor.entity_count())
}
"#;

    let output = compile_and_run_wj(code).expect("Should compile and run");
    assert!(
        output.contains("Created entity: 1"),
        "Should create entity with ID 1"
    );
    assert!(
        output.contains("Created entity: 2"),
        "Should create entity with ID 2"
    );
    assert!(
        output.contains("Created entity: 3"),
        "Should create entity with ID 3"
    );
    assert!(
        output.contains("Total entities: 3"),
        "Should have 3 entities"
    );
}

#[test]
fn test_scene_editor_select_entity() {
    let code = r#"
struct EditorState {
    pub next_entity_id: i64,
    pub entities: Vec<i64>,
    pub selected_entity: Option<i64>,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            next_entity_id: 1,
            entities: Vec::new(),
            selected_entity: None,
        }
    }

    pub fn create_entity(&mut self) -> i64 {
        let id = self.next_entity_id
        self.next_entity_id = self.next_entity_id + 1
        self.entities.push(id)
        id
    }

    pub fn select_entity(&mut self, entity_id: i64) {
        self.selected_entity = Some(entity_id)
    }

    pub fn deselect(&mut self) {
        self.selected_entity = None
    }

    pub fn is_selected(&self, entity_id: i64) -> bool {
        if self.selected_entity.is_some() {
            self.selected_entity.unwrap() == entity_id
        } else {
            false
        }
    }
}

fn main() {
    let mut editor = EditorState::new()
    
    let entity1 = editor.create_entity()
    let entity2 = editor.create_entity()
    
    editor.select_entity(entity1)
    println!("Entity 1 selected: {}", editor.is_selected(entity1))
    println!("Entity 2 selected: {}", editor.is_selected(entity2))
    
    editor.select_entity(entity2)
    println!("Entity 1 selected after switch: {}", editor.is_selected(entity1))
    println!("Entity 2 selected after switch: {}", editor.is_selected(entity2))
    
    editor.deselect()
    println!("Entity 1 selected after deselect: {}", editor.is_selected(entity1))
    println!("Entity 2 selected after deselect: {}", editor.is_selected(entity2))
}
"#;

    let output = compile_and_run_wj(code).expect("Should compile and run");
    assert!(
        output.contains("Entity 1 selected: true"),
        "Entity 1 should be selected"
    );
    assert!(
        output.contains("Entity 2 selected: false"),
        "Entity 2 should not be selected"
    );
    assert!(
        output.contains("Entity 1 selected after switch: false"),
        "Entity 1 should not be selected after switch"
    );
    assert!(
        output.contains("Entity 2 selected after switch: true"),
        "Entity 2 should be selected after switch"
    );
    assert!(
        output.contains("Entity 1 selected after deselect: false"),
        "Entity 1 should not be selected after deselect"
    );
    assert!(
        output.contains("Entity 2 selected after deselect: false"),
        "Entity 2 should not be selected after deselect"
    );
}

#[test]
fn test_scene_editor_delete_entity() {
    let code = r#"
struct EditorState {
    pub next_entity_id: i64,
    pub entities: Vec<i64>,
    pub selected_entity: Option<i64>,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            next_entity_id: 1,
            entities: Vec::new(),
            selected_entity: None,
        }
    }

    pub fn create_entity(&mut self) -> i64 {
        let id = self.next_entity_id
        self.next_entity_id = self.next_entity_id + 1
        self.entities.push(id)
        id
    }

    pub fn delete_entity(&mut self, entity_id: i64) {
        self.entities.retain(|id| *id != entity_id)
        
        // Deselect if deleted entity was selected
        if self.selected_entity.is_some() {
            if self.selected_entity.unwrap() == entity_id {
                self.selected_entity = None
            }
        }
    }

    pub fn entity_count(&self) -> i64 {
        self.entities.len() as i64
    }

    pub fn select_entity(&mut self, entity_id: i64) {
        self.selected_entity = Some(entity_id)
    }

    pub fn has_selection(&self) -> bool {
        self.selected_entity.is_some()
    }
}

fn main() {
    let mut editor = EditorState::new()
    
    let entity1 = editor.create_entity()
    let entity2 = editor.create_entity()
    let entity3 = editor.create_entity()
    
    println!("Initial count: {}", editor.entity_count())
    
    editor.select_entity(entity2)
    println!("Has selection before delete: {}", editor.has_selection())
    
    editor.delete_entity(entity2)
    println!("Count after delete: {}", editor.entity_count())
    println!("Has selection after delete: {}", editor.has_selection())
}
"#;

    let output = compile_and_run_wj(code).expect("Should compile and run");
    assert!(
        output.contains("Initial count: 3"),
        "Should start with 3 entities"
    );
    assert!(
        output.contains("Has selection before delete: true"),
        "Should have selection before delete"
    );
    assert!(
        output.contains("Count after delete: 2"),
        "Should have 2 entities after delete"
    );
    assert!(
        output.contains("Has selection after delete: false"),
        "Should not have selection after deleting selected entity"
    );
}

#[test]
fn test_scene_editor_multi_select() {
    let code = r#"
struct EditorState {
    pub next_entity_id: i64,
    pub entities: Vec<i64>,
    pub selected_entities: Vec<i64>,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            next_entity_id: 1,
            entities: Vec::new(),
            selected_entities: Vec::new(),
        }
    }

    pub fn create_entity(&mut self) -> i64 {
        let id = self.next_entity_id
        self.next_entity_id = self.next_entity_id + 1
        self.entities.push(id)
        id
    }

    pub fn select_entity(&mut self, entity_id: i64) {
        self.selected_entities.clear()
        self.selected_entities.push(entity_id)
    }

    pub fn add_to_selection(&mut self, entity_id: i64) {
        if !self.is_selected(entity_id) {
            self.selected_entities.push(entity_id)
        }
    }

    pub fn remove_from_selection(&mut self, entity_id: i64) {
        self.selected_entities.retain(|id| *id != entity_id)
    }

    pub fn is_selected(&self, entity_id: i64) -> bool {
        for id in self.selected_entities.iter() {
            if *id == entity_id {
                return true
            }
        }
        false
    }

    pub fn selection_count(&self) -> i64 {
        self.selected_entities.len() as i64
    }

    pub fn clear_selection(&mut self) {
        self.selected_entities.clear()
    }
}

fn main() {
    let mut editor = EditorState::new()
    
    let entity1 = editor.create_entity()
    let entity2 = editor.create_entity()
    let entity3 = editor.create_entity()
    
    editor.select_entity(entity1)
    println!("Selection count after single select: {}", editor.selection_count())
    
    editor.add_to_selection(entity2)
    editor.add_to_selection(entity3)
    println!("Selection count after multi-select: {}", editor.selection_count())
    println!("Entity 1 selected: {}", editor.is_selected(entity1))
    println!("Entity 2 selected: {}", editor.is_selected(entity2))
    println!("Entity 3 selected: {}", editor.is_selected(entity3))
    
    editor.remove_from_selection(entity2)
    println!("Selection count after removal: {}", editor.selection_count())
    println!("Entity 2 selected after removal: {}", editor.is_selected(entity2))
    
    editor.clear_selection()
    println!("Selection count after clear: {}", editor.selection_count())
}
"#;

    let output = compile_and_run_wj(code).expect("Should compile and run");
    assert!(
        output.contains("Selection count after single select: 1"),
        "Should have 1 entity selected"
    );
    assert!(
        output.contains("Selection count after multi-select: 3"),
        "Should have 3 entities selected"
    );
    assert!(
        output.contains("Entity 1 selected: true"),
        "Entity 1 should be selected"
    );
    assert!(
        output.contains("Entity 2 selected: true"),
        "Entity 2 should be selected"
    );
    assert!(
        output.contains("Entity 3 selected: true"),
        "Entity 3 should be selected"
    );
    assert!(
        output.contains("Selection count after removal: 2"),
        "Should have 2 entities selected after removal"
    );
    assert!(
        output.contains("Entity 2 selected after removal: false"),
        "Entity 2 should not be selected after removal"
    );
    assert!(
        output.contains("Selection count after clear: 0"),
        "Should have 0 entities selected after clear"
    );
}
