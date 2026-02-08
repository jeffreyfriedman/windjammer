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

// ============================================================================
// TEST 5: Trait Method Self Inference Gap
//
// Real game code: systems.wj defines a System trait with abstract methods:
//   fn name(self) -> string
//   fn update(self, dt: f32)
//   fn is_enabled(self) -> bool
//   fn priority(self) -> i32 { 0 }  // has default body
//
// The compiler generates bare `self` for abstract methods (name, update, is_enabled)
// but correctly generates `&self` for priority (which has a default body).
//
// This is a compiler bug: abstract trait methods should NEVER use bare `self`
// (which moves ownership) unless explicitly requested. The compiler should:
//   - Default abstract methods to `&self` (borrowed)
//   - Upgrade to `&mut self` if ANY impl body mutates self for that method
//   - Impls must match the inferred trait signature
//
// This tests the Windjammer philosophy: the compiler infers ownership, not the user.
// ============================================================================

#[test]
fn test_trait_abstract_method_self_inference() {
    let source = r#"
pub trait System {
    fn name(self) -> string
    fn update(self, dt: f32)
    fn is_enabled(self) -> bool
    fn priority(self) -> i32 {
        0
    }
}

pub struct PhysicsSystem {
    enabled: bool,
    gravity: f32,
}

impl PhysicsSystem {
    pub fn new(gravity: f32) -> PhysicsSystem {
        PhysicsSystem { enabled: true, gravity: gravity }
    }
}

impl System for PhysicsSystem {
    fn name(self) -> string {
        "PhysicsSystem".to_string()
    }
    fn update(self, dt: f32) {
        // read-only: just reads self.gravity
        let force = self.gravity * dt
    }
    fn is_enabled(self) -> bool {
        self.enabled
    }
    fn priority(self) -> i32 {
        100
    }
}

pub struct RenderSystem {
    enabled: bool,
    draw_calls: i32,
}

impl RenderSystem {
    pub fn new() -> RenderSystem {
        RenderSystem { enabled: true, draw_calls: 0 }
    }
}

impl System for RenderSystem {
    fn name(self) -> string {
        "RenderSystem".to_string()
    }
    fn update(self, dt: f32) {
        // MUTATES self: resets draw_calls
        self.draw_calls = 0
    }
    fn is_enabled(self) -> bool {
        self.enabled
    }
    fn priority(self) -> i32 {
        -100
    }
}
"#;

    let (generated, stderr) = compile_wj(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // CRITICAL: Abstract trait methods must NOT use bare `self`
    // bare `self` moves ownership, making the trait non-object-safe
    // and preventing calling the method more than once on the same object

    // The trait definition should use &self for read-only methods
    assert!(
        generated.contains("fn name(&self) -> String"),
        "COMPILER BUG: Trait method 'name' should be '&self' (read-only in all impls).\n\
         Bare 'self' moves ownership, which is almost never intended for trait methods.\n\
         Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn is_enabled(&self) -> bool"),
        "COMPILER BUG: Trait method 'is_enabled' should be '&self' (read-only in all impls).\n\
         Generated:\n{}",
        generated
    );

    // update() should be &mut self because RenderSystem::update mutates self.draw_calls
    assert!(
        generated.contains("fn update(&mut self, dt: f32)"),
        "COMPILER BUG: Trait method 'update' should be '&mut self' because RenderSystem::update \
         mutates self.draw_calls.\n\
         Generated:\n{}",
        generated
    );

    // priority() has a default body and should be &self (read-only default)
    assert!(
        generated.contains("fn priority(&self) -> i32"),
        "COMPILER BUG: Trait method 'priority' should be '&self' (default impl is read-only).\n\
         Generated:\n{}",
        generated
    );

    // Impl methods must match the trait signature
    // PhysicsSystem::update should be &mut self (matching trait, even though body is read-only)
    assert!(
        generated.contains("fn update(&mut self, dt: f32)"),
        "COMPILER BUG: Impl method 'update' must match trait signature '&mut self'.\n\
         Generated:\n{}",
        generated
    );

    // name and is_enabled impls should be &self (matching trait)
    // Count occurrences to verify both trait AND impl use &self
    let name_ref_count = generated.matches("fn name(&self) -> String").count();
    assert!(
        name_ref_count >= 2,
        "COMPILER BUG: Expected at least 2 occurrences of 'fn name(&self) -> String' \
         (trait + impls), found {}.\nGenerated:\n{}",
        name_ref_count,
        generated
    );
}

// ============================================================================
// TEST 6: Trailing Semicolon on Return Expressions in Default Trait Methods
//
// In Rust, the last expression in a block must NOT have a trailing semicolon
// if it's the return value. `fn priority(&self) -> i32 { 0; }` is a type error
// because `0;` evaluates to `()` (unit), not `i32`.
//
// The correct code is `fn priority(&self) -> i32 { 0 }` (no semicolon).
//
// This affects default trait methods, match arm bodies, if/else return values,
// and any block where the last expression is the implicit return.
//
// THE WINDJAMMER WAY: The compiler generates correct Rust code.
// ============================================================================

#[test]
fn test_default_trait_method_return_no_trailing_semicolon() {
    let source = r#"
pub trait Configurable {
    fn default_value(self) -> i32 {
        42
    }

    fn default_name(self) -> string {
        "unnamed".to_string()
    }

    fn is_valid(self) -> bool {
        true
    }

    fn compute(self, x: f32) -> f32 {
        x * 2.0
    }
}
"#;

    let (generated, stderr) = compile_wj(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // The return expression must NOT have a trailing semicolon
    // Good: `fn default_value(&self) -> i32 { 42 }`
    // Bad:  `fn default_value(&self) -> i32 { 42; }`

    // Check that 42 is returned without semicolon
    assert!(
        generated.contains("42\n") || generated.contains("42 }") || generated.contains("42\r\n"),
        "COMPILER BUG: Return expression '42' in default trait method should NOT have trailing semicolon.\n\
         `42;` evaluates to `()`, not `i32`, causing E0308.\n\
         Generated:\n{}",
        generated
    );

    // Specifically check it doesn't have the bad pattern
    // Find the default_value method body and check the return expression
    let has_semicolon_42 = generated.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "42;" && !trimmed.starts_with("//")
    });
    assert!(
        !has_semicolon_42,
        "COMPILER BUG: Found '42;' with trailing semicolon in default trait method.\n\
         This causes E0308: expected `i32`, found `()`.\n\
         Generated:\n{}",
        generated
    );

    // Check that true is returned without semicolon
    let has_semicolon_true = generated.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "true;" && !trimmed.starts_with("//")
    });
    assert!(
        !has_semicolon_true,
        "COMPILER BUG: Found 'true;' with trailing semicolon in default trait method.\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST: Auto-wrap function pointers in iterator adapters
//
// Real game code pattern (from ecs/query_system.wj, event/dispatcher.wj):
//   pub fn count_where(self, predicate: fn(&Entity) -> bool) -> usize {
//       self.entities.iter().filter(predicate).count()
//   }
//
// BUG: Rust's .filter() on iter() yields &&Entity, but predicate expects &Entity.
// Bare function pointers don't auto-deref, causing E0631.
//
// THE WINDJAMMER WAY: The compiler automatically wraps bare function pointers
// in closures when passed to iterator adapters. The user writes the natural
// `filter(predicate)` and the compiler generates `filter(|__e| predicate(__e))`.
// ============================================================================

#[test]
fn test_auto_wrap_function_pointer_in_iterator_adapter() {
    let source = r#"
pub struct Item {
    pub name: string,
    pub value: i32,
    pub active: bool,
}

pub struct Inventory {
    items: Vec<Item>,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory { items: Vec::new() }
    }

    /// Count items matching a predicate - uses filter with function pointer
    pub fn count_where(self, predicate: fn(&Item) -> bool) -> usize {
        self.items.iter().filter(predicate).count()
    }

    /// Check if any item matches - uses any with function pointer
    pub fn any_match(self, predicate: fn(&Item) -> bool) -> bool {
        self.items.iter().any(predicate)
    }

    /// Find first matching item - uses find with function pointer
    pub fn find_item(self, predicate: fn(&Item) -> bool) -> Option<&Item> {
        self.items.iter().find(predicate)
    }

    /// Count items NOT matching - uses filter with negated function pointer
    pub fn count_not_matching(self, predicate: fn(&Item) -> bool) -> usize {
        self.items.iter().filter(|e| !predicate(e)).count()
    }
}
"#;

    let (generated, stderr) = compile_wj(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // The key check: bare function pointers passed to .filter(), .any(), .find()
    // must be wrapped in closures, NOT passed directly.
    //
    // GOOD: .filter(|__e| predicate(__e))
    // BAD:  .filter(predicate)
    //
    // Bare `predicate` causes E0631 because .filter() on iter() expects
    // FnMut(&&Item) -> bool, but fn(&Item) -> bool doesn't match.

    // Check filter(predicate) is wrapped
    assert!(
        !generated.contains(".filter(predicate)"),
        "COMPILER BUG: Bare function pointer 'predicate' passed to .filter().\n\
         This causes E0631 because .filter() on iter() expects FnMut(&&Item),\n\
         but fn(&Item) doesn't auto-deref. Must wrap: .filter(|__e| predicate(__e))\n\
         Generated:\n{}",
        generated
    );

    // Check any(predicate) is wrapped
    assert!(
        !generated.contains(".any(predicate)"),
        "COMPILER BUG: Bare function pointer 'predicate' passed to .any().\n\
         Must wrap: .any(|__e| predicate(__e))\n\
         Generated:\n{}",
        generated
    );

    // Check find(predicate) is wrapped
    assert!(
        !generated.contains(".find(predicate)"),
        "COMPILER BUG: Bare function pointer 'predicate' passed to .find().\n\
         Must wrap: .find(|__e| predicate(__e))\n\
         Generated:\n{}",
        generated
    );

    // Verify the closure wrapper pattern exists (any reasonable wrapping)
    // The pattern should be something like |__e| predicate(__e) or |e| predicate(e)
    let has_filter_wrapper = generated.contains(".filter(|")
        && generated.contains("predicate(");
    assert!(
        has_filter_wrapper,
        "COMPILER BUG: Expected .filter() to have a closure wrapper around 'predicate'.\n\
         Generated:\n{}",
        generated
    );

    let has_any_wrapper = generated.contains(".any(|")
        && (generated.matches("predicate(").count() >= 2);  // at least filter + any
    assert!(
        has_any_wrapper,
        "COMPILER BUG: Expected .any() to have a closure wrapper around 'predicate'.\n\
         Generated:\n{}",
        generated
    );

    // The manually-written closure in count_not_matching should pass through unchanged
    // (it already IS a closure, no wrapping needed)
    assert!(
        generated.contains("|e| !predicate(e)") || generated.contains("|e| !(predicate(e))"),
        "Manually written closure should pass through unchanged.\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST: Auto-infer mut for parameters when mutating methods called on them
//
// Real game code pattern (from assets/loader.wj):
//   pub fn load_game_level(loader: AssetLoader, level_name: string) -> ... {
//       let tilemap = loader.load(...)
//       let texture = loader.load(...)
//   }
//
// BUG: .load() takes &mut self, so Rust needs `mut loader: AssetLoader`.
// Without `mut`, Rust gives E0596: cannot borrow `loader` as mutable.
//
// THE WINDJAMMER WAY: The compiler automatically infers `mut` for parameters
// when methods requiring `&mut self` are called on them. The user writes
// the clean `loader: AssetLoader` and the compiler handles mutability.
// ============================================================================

#[test]
fn test_auto_infer_mut_for_parameter_with_mutating_method_calls() {
    let source = r#"
pub struct ResourcePool {
    items: Vec<string>,
    count: i32,
}

impl ResourcePool {
    pub fn new() -> ResourcePool {
        ResourcePool { items: Vec::new(), count: 0 }
    }

    /// This method mutates self (pushes to items, increments count)
    pub fn add(self, item: string) {
        self.items.push(item)
        self.count = self.count + 1
    }

    /// This method only reads self
    pub fn size(self) -> i32 {
        self.count
    }
}

/// Function that takes owned ResourcePool and calls mutating methods on it
/// THE WINDJAMMER WAY: User writes `pool: ResourcePool` without `mut`,
/// compiler infers `mut` because .add() requires &mut self
pub fn fill_pool(pool: ResourcePool) {
    pool.add("water".to_string())
    pool.add("food".to_string())
    pool.add("ammo".to_string())
}

/// Function that takes owned ResourcePool but only reads it
/// Should NOT get mut
pub fn check_pool(pool: ResourcePool) -> i32 {
    pool.size()
}
"#;

    let (generated, stderr) = compile_wj(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // fill_pool: pool has .add() called on it, which requires &mut self.
    // The parameter binding needs `mut`.
    // GOOD: fn fill_pool(mut pool: ResourcePool)
    // BAD:  fn fill_pool(pool: ResourcePool)
    assert!(
        generated.contains("mut pool: ResourcePool"),
        "COMPILER BUG: Parameter 'pool' should be inferred as 'mut pool' because\n\
         .add() is called on it, which requires &mut self. Without 'mut',\n\
         Rust gives E0596: cannot borrow `pool` as mutable.\n\
         Generated:\n{}",
        generated
    );

    // check_pool: pool only has .size() called, which requires &self (read-only).
    // The parameter should NOT have `mut`.
    // We check that check_pool's signature does not have `mut pool`
    let check_pool_line = generated.lines().find(|l| l.contains("fn check_pool"));
    if let Some(line) = check_pool_line {
        assert!(
            !line.contains("mut pool"),
            "Read-only parameter 'pool' in check_pool should NOT be inferred as 'mut'.\n\
             Line: {}\n\
             Generated:\n{}",
            line,
            generated
        );
    }
}
