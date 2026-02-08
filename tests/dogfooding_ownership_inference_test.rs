/// Dogfooding ownership inference tests
///
/// These tests reproduce REAL bugs found by compiling the Windjammer game engine.
/// Each test mirrors actual game engine code patterns where the compiler incorrectly
/// adds & to arguments that should be passed by value.
///
/// THE WINDJAMMER PHILOSOPHY: The compiler handles ownership, not the user.
/// Users write clean code like `Quest::new(id, title, description)` and the
/// compiler figures out the correct Rust ownership annotations.
///
/// Bug categories reproduced here:
/// 1. Factory pattern: Quest::new(id, title, desc) generates &title, &description
/// 2. Copy type in match-bound context: stack.remove(to_remove) generates &to_remove
/// 3. Copy type self.field passed to method: transition.matches(self.current_state) generates &self.current_state
/// 4. Vec::remove with usize variable: self.dense.remove(idx) generates &idx
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj(source: &str) -> (String, String) {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    let stderr = String::from_utf8_lossy(&wj_output.stderr).to_string();
    (generated, stderr)
}

// ============================================================================
// TEST 1: Factory Pattern - Quest system (narrative/quest.wj)
//
// Real game code: Quest::new(id, title, description) where all are String
// The factory function new_main_quest passes String params directly to Quest::new.
// The compiler should NOT add & to these arguments.
// ============================================================================

#[test]
fn test_factory_pattern_string_passthrough() {
    let source = r#"
pub struct Quest {
    pub id: string,
    pub title: string,
    pub description: string,
    pub is_main_quest: bool,
    pub is_side_quest: bool,
}

impl Quest {
    pub fn new(id: string, title: string, description: string) -> Quest {
        Quest {
            id: id,
            title: title,
            description: description,
            is_main_quest: false,
            is_side_quest: true,
        }
    }

    pub fn new_main_quest(id: string, title: string, description: string) -> Quest {
        let mut quest = Quest::new(id, title, description)
        quest.is_main_quest = true
        quest.is_side_quest = false
        quest
    }
}
"#;

    let (generated, _) = compile_wj(source);

    // The factory function should pass String args directly, NOT with &
    assert!(
        !generated.contains("Quest::new(id, &title, &description)"),
        "COMPILER BUG: Quest::new should NOT add & to String params.\n\
         The user wrote: Quest::new(id, title, description)\n\
         Compiler generated: Quest::new(id, &title, &description)\n\
         Generated:\n{}",
        generated
    );

    // Should contain the correct call (without &)
    assert!(
        generated.contains("Quest::new(id, title, description)")
            || generated.contains("Quest::new(id.clone(), title, description)")
            || generated.contains("Quest::new(id, title.clone(), description")
            // The compiler might add .clone() for moved values, which is acceptable
            || generated.contains("Quest::new(id, title.to_string(), description.to_string()"),
        "Expected Quest::new with owned String args (no &). Got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 2: Factory with format! - Quest system (narrative/quest.wj)
//
// Real game code: Quest::new(id.clone(), title, format!("Collect {} {}", quantity, item_id))
// The format! macro produces an owned String. The compiler should NOT add & to it.
// ============================================================================

#[test]
fn test_factory_pattern_format_arg() {
    let source = r#"
pub struct Quest {
    pub id: string,
    pub title: string,
    pub description: string,
}

impl Quest {
    pub fn new(id: string, title: string, description: string) -> Quest {
        Quest {
            id: id,
            title: title,
            description: description,
        }
    }
}

pub fn create_fetch_quest(id: string, title: string, item_id: string, quantity: i32) -> Quest {
    let mut quest = Quest::new(id.clone(), title, format!("Collect {} {}", quantity, item_id))
    quest
}
"#;

    let (generated, _) = compile_wj(source);

    // format! returns owned String - should NOT be &format!(...)
    assert!(
        !generated.contains("&format!"),
        "COMPILER BUG: format!() already returns String, should NOT add &.\n\
         Generated:\n{}",
        generated
    );

    // title should not be &title either
    assert!(
        !generated.contains("&title"),
        "COMPILER BUG: title is String passed to String param, should NOT add &.\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 3: Copy type in match-bound context - Inventory system (inventory/inventory.wj)
//
// Real game code: Inside `if let Some(stack) = slot`, calling stack.remove(to_remove)
// where to_remove is u32 (Copy type from .min() call).
// The compiler should NOT add & to Copy type arguments.
// ============================================================================

#[test]
fn test_copy_type_in_match_bound_method_call() {
    let source = r#"
pub struct ItemStack {
    quantity: u32,
}

impl ItemStack {
    pub fn remove(amount: u32) {
        self.quantity = self.quantity - amount
    }

    pub fn quantity() -> u32 {
        self.quantity
    }

    pub fn is_empty() -> bool {
        self.quantity == 0
    }
}

pub fn remove_items(slots: &mut Vec<Option<ItemStack>>, quantity: u32) -> u32 {
    let mut removed: u32 = 0
    let mut remaining: u32 = quantity

    for slot in slots {
        if remaining == 0 {
            break
        }

        if let Some(stack) = slot {
            let to_remove = remaining.min(stack.quantity())
            stack.remove(to_remove)
            removed = removed + to_remove
            remaining = remaining - to_remove
        }
    }

    removed
}
"#;

    let (generated, _) = compile_wj(source);

    // to_remove is u32 (Copy), should NOT be &to_remove
    assert!(
        !generated.contains("stack.remove(&to_remove)"),
        "COMPILER BUG: to_remove is u32 (Copy type), should NOT add &.\n\
         The user wrote: stack.remove(to_remove)\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 4: Copy type self.field passed to method - AI system (ai/state_machine.wj)
//
// Real game code: transition.matches(self.current_state)
// where current_state is i32 (Copy type).
// The compiler should NOT add & to self.field when it's a Copy type.
// ============================================================================

#[test]
fn test_self_field_copy_type_passed_to_method() {
    let source = r#"
pub struct Transition {
    pub from_state: i32,
    pub to_state: i32,
}

impl Transition {
    pub fn matches(state: i32) -> bool {
        self.from_state == state
    }
}

pub struct StateMachine {
    pub current_state: i32,
    pub transitions: Vec<Transition>,
}

impl StateMachine {
    pub fn should_transition(to_state: i32) -> bool {
        for transition in &self.transitions {
            if transition.matches(self.current_state) && transition.to_state == to_state {
                return true
            }
        }
        false
    }
}
"#;

    let (generated, _) = compile_wj(source);

    // self.current_state is i32 (Copy), should NOT be &self.current_state
    assert!(
        !generated.contains("transition.matches(&self.current_state)"),
        "COMPILER BUG: self.current_state is i32 (Copy), should NOT add &.\n\
         The user wrote: transition.matches(self.current_state)\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 5: Vec::remove with usize variable - ECS system (ecs/components.wj)
//
// Real game code: self.dense.remove(sparse_idx_usize)
// where sparse_idx_usize is explicitly typed as usize.
// Vec::remove takes usize by value, NOT by reference.
// ============================================================================

#[test]
fn test_vec_remove_usize_variable() {
    let source = r#"
pub struct SparseSet {
    sparse: Vec<int>,
    dense: Vec<int>,
    entities: Vec<int>,
}

impl SparseSet {
    pub fn remove(entity_index: usize) -> Option<int> {
        if entity_index >= self.sparse.len() {
            return None
        }

        let sparse_index: int = self.sparse[entity_index]
        if sparse_index < 0 {
            return None
        }

        let sparse_idx_usize: usize = sparse_index as usize
        let component = self.dense.remove(sparse_idx_usize)
        let removed_entity = self.entities.remove(sparse_idx_usize)

        self.sparse[entity_index] = -1

        Some(component)
    }
}
"#;

    let (generated, _) = compile_wj(source);

    // sparse_idx_usize is usize, Vec::remove takes usize by value
    assert!(
        !generated.contains(".remove(&sparse_idx_usize)"),
        "COMPILER BUG: sparse_idx_usize is usize, Vec::remove takes by value.\n\
         The user wrote: self.dense.remove(sparse_idx_usize)\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 6: Cross-type mutation inference via self.field.method()
//         (terrain/terrain.wj + terrain/heightmap.wj)
//
// Real game code: Terrain::raise() calls self.heightmap.set(px, pz, value)
// HeightMap::set mutates self.data[...], so it requires &mut self.
// Since Terrain::raise calls self.heightmap.set(), it requires &mut self too.
// The compiler must infer &mut self for BOTH HeightMap::set and Terrain::raise.
//
// This tests cross-type mutation propagation through self.field.method().
// ============================================================================

#[test]
fn test_cross_type_mutation_via_field_method() {
    let source = r#"
pub struct HeightMap {
    width: usize,
    height: usize,
    data: Vec<f32>,
}

impl HeightMap {
    pub fn new(width: usize, height: usize) -> HeightMap {
        let data = Vec::with_capacity(width * height)
        HeightMap {
            width: width,
            height: height,
            data: data,
        }
    }

    pub fn get(self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            0.0
        }
    }

    pub fn set(self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value
        }
    }
}

pub struct Terrain {
    heightmap: HeightMap,
    scale: f32,
}

impl Terrain {
    pub fn new(width: usize, height: usize, scale: f32) -> Terrain {
        Terrain {
            heightmap: HeightMap::new(width, height),
            scale: scale,
        }
    }

    pub fn get_height(self, x: f32, z: f32) -> f32 {
        let grid_x = (x / self.scale) as usize
        let grid_z = (z / self.scale) as usize
        self.heightmap.get(grid_x, grid_z)
    }

    pub fn raise(self, x: f32, z: f32, radius: f32, strength: f32) {
        let grid_x = (x / self.scale) as i32
        let grid_z = (z / self.scale) as i32
        let grid_radius = (radius / self.scale) as i32

        let mut dz = -grid_radius
        while dz <= grid_radius {
            let mut dx = -grid_radius
            while dx <= grid_radius {
                let px = (grid_x + dx) as usize
                let pz = (grid_z + dz) as usize

                let current = self.heightmap.get(px, pz)
                self.heightmap.set(px, pz, current + strength)

                dx = dx + 1
            }
            dz = dz + 1
        }
    }

    pub fn lower(self, x: f32, z: f32, radius: f32, strength: f32) {
        self.raise(x, z, radius, -strength)
    }

    pub fn clear(self) {
        self.heightmap.clear()
    }
}
"#;

    let (generated, _) = compile_wj(source);

    // HeightMap::set mutates self.data[...], so it MUST be &mut self
    assert!(
        generated.contains("pub fn set(&mut self"),
        "COMPILER BUG: HeightMap::set mutates self.data[idx], should be &mut self.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::raise calls self.heightmap.set(), so it MUST be &mut self
    assert!(
        generated.contains("pub fn raise(&mut self"),
        "COMPILER BUG: Terrain::raise calls self.heightmap.set() which mutates.\n\
         The compiler must propagate mutation through self.field.method() calls.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::lower calls self.raise(), so it MUST be &mut self
    assert!(
        generated.contains("pub fn lower(&mut self"),
        "COMPILER BUG: Terrain::lower calls self.raise() which is &mut self.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::clear calls self.heightmap.clear(), which is a known mutating method
    assert!(
        generated.contains("pub fn clear(&mut self"),
        "COMPILER BUG: Terrain::clear calls self.heightmap.clear() which mutates.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::get_height only reads, so it should be &self
    assert!(
        generated.contains("pub fn get_height(&self"),
        "REGRESSION: Terrain::get_height only reads, should be &self.\n\
         Generated:\n{}",
        generated
    );

    // HeightMap::get only reads, so it should be &self
    assert!(
        generated.contains("pub fn get(&self"),
        "REGRESSION: HeightMap::get only reads, should be &self.\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 7: Borrow conflict in match on self.method() with arm mutation (E0502)
//         (scene/manager.wj)
//
// Real game code: pop_scene() calls self.current_scene_id() in a match,
// then self.paused_scenes.remove(current) in the arm body.
// current_scene_id() returns Option<&str> which borrows self immutably.
// remove() needs &mut self.paused_scenes, creating a borrow conflict.
//
// The compiler must detect this pattern and break the borrow by extracting
// the scrutinee into an owned temporary:
//   let __match_borrow_break = self.current_scene_id().map(|v| v.to_owned());
//   match __match_borrow_break.as_deref() { ... }
// ============================================================================

#[test]
fn test_borrow_conflict_match_self_method_with_arm_mutation() {
    let source = r#"
pub struct SceneManager {
    scene_stack: Vec<String>,
    paused_scenes: HashSet<String>,
}

impl SceneManager {
    pub fn current_scene_id(self) -> Option<&str> {
        if self.scene_stack.len() > 0 {
            Some(&self.scene_stack[self.scene_stack.len() - 1])
        } else {
            None
        }
    }

    pub fn pop_scene(self) {
        if self.scene_stack.len() > 0 {
            let popped = self.scene_stack.remove(self.scene_stack.len() - 1)
            self.paused_scenes.remove(&popped)

            if let Some(current) = self.current_scene_id() {
                self.paused_scenes.remove(current)
            }
        }
    }
}
"#;

    let (generated, _) = compile_wj(source);

    // The match on self.current_scene_id() should NOT directly appear as the match scrutinee
    // when the arm body mutates self. Instead, the compiler should extract into an owned temp.
    assert!(
        !generated.contains("match self.current_scene_id()"),
        "COMPILER BUG: match on self.current_scene_id() with arm mutation causes E0502.\n\
         The compiler should extract the scrutinee into an owned temporary.\n\
         Generated:\n{}",
        generated
    );

    // Should have the borrow-break pattern
    assert!(
        generated.contains("__match_borrow_break")
            || generated.contains(".map(|__v| __v.to_owned())")
            || generated.contains(".as_deref()"),
        "COMPILER BUG: Expected borrow-break pattern for match on self.method() with arm mutation.\n\
         Generated:\n{}",
        generated
    );

    // pop_scene should be &mut self (it mutates scene_stack and paused_scenes)
    assert!(
        generated.contains("pub fn pop_scene(&mut self"),
        "COMPILER BUG: pop_scene mutates fields, should be &mut self.\n\
         Generated:\n{}",
        generated
    );
}
