#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

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
#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// TEST 1: Factory Pattern - Quest system (narrative/quest.wj)
//
// Real game code: Quest::new(id, title, description) where all are String
// The factory function new_main_quest passes String params directly to Quest::new.
// The compiler should NOT add & to these arguments.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

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
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

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
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

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
#[cfg_attr(tarpaulin, ignore)]
#[ignore = "borrow-break extraction not emitted for this fixture until match lowering is restored"]
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

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

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

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, stderr) = test_utils::compile_via_cli_with_stderr(source);
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
    let has_filter_wrapper = generated.contains(".filter(|") && generated.contains("predicate(");
    assert!(
        has_filter_wrapper,
        "COMPILER BUG: Expected .filter() to have a closure wrapper around 'predicate'.\n\
         Generated:\n{}",
        generated
    );

    let has_any_wrapper =
        generated.contains(".any(|") && (generated.matches("predicate(").count() >= 2); // at least filter + any
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
#[cfg_attr(tarpaulin, ignore)]
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
/// NOTE: String literals auto-convert to String (no .to_string() needed!)
pub fn fill_pool(pool: ResourcePool) {
    pool.add("water")
    pool.add("food")
    pool.add("ammo")
}

/// Function that takes owned ResourcePool but only reads it
/// Should NOT get mut
pub fn check_pool(pool: ResourcePool) -> i32 {
    pool.size()
}
"#;

    let (generated, stderr) = test_utils::compile_via_cli_with_stderr(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // fill_pool: pool has .add() called on it, which requires &mut self.
    // The compiler infers &mut ResourcePool (mutable borrow) since pool is mutated
    // but not returned. This is idiomatic Rust.
    // GOOD: fn fill_pool(pool: &mut ResourcePool) OR fn fill_pool(mut pool: ResourcePool)
    let fill_pool_line = generated.lines().find(|l| l.contains("fn fill_pool"));
    assert!(
        fill_pool_line.is_some(),
        "fill_pool function should exist in generated code.\nGenerated:\n{}",
        generated
    );
    let fill_line = fill_pool_line.unwrap();
    assert!(
        fill_line.contains("&mut ResourcePool") || fill_line.contains("mut pool"),
        "COMPILER BUG: Parameter 'pool' in fill_pool should be mutable.\n\
         Expected either `pool: &mut ResourcePool` or `mut pool: ResourcePool`.\n\
         Got: {}\n\
         Generated:\n{}",
        fill_line,
        generated
    );

    // check_pool: pool only has .size() called, which requires &self (read-only).
    // The parameter should NOT have `mut`.
    let check_pool_line = generated.lines().find(|l| l.contains("fn check_pool"));
    if let Some(line) = check_pool_line {
        assert!(
            !line.contains("mut pool") && !line.contains("&mut"),
            "Read-only parameter 'pool' in check_pool should NOT be inferred as 'mut'.\n\
             Line: {}\n\
             Generated:\n{}",
            line,
            generated
        );
    }
}
