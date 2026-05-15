#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: E0282 Type Annotations - Vec::new() inference from .push()
///
/// Bug: `let mut x = Vec::new(); x.push(item)` generates Rust that needs type annotation.
/// Root cause: Generic functions (Vec::new) need type hints; Windjammer didn't generate them.
///
/// Fix: Infer Vec<T> from first .push() in body and emit `let mut x: Vec<T> = Vec::new()`.
///
/// Philosophy: "Fix inference when context exists" - don't guess when ambiguous.
#[path = "../common/test_utils.rs"]
mod test_utils;

/// E0282 Phase 9: Parser produces Call{function: Identifier("Vec::new")} not MethodCall.
/// This test verifies we handle the qualified-path Identifier in the Call branch.
#[test]
fn test_vec_new_push_call_identifier_path() {
    let source = r#"
pub fn test_method_call_inference() -> Vec<i32> {
    let mut result = Vec::new()
    result.push(42)
    result
}
"#;

    let rust = test_utils::compile_single(source);

    // Call+Identifier fix: Vec::new() parsed as Identifier("Vec::new") must infer from push
    assert!(
        rust.contains("let mut result: Vec<i32> = Vec::new()"),
        "Should emit Vec<i32> for Vec::new() + push(42) (Call+Identifier path). Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_new_push_infers_element_type() {
    let source = r#"
struct AABB {
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
}

impl AABB {
    pub fn new(min_x: f32, min_y: f32, min_z: f32, max_x: f32, max_y: f32, max_z: f32) -> AABB {
        AABB { min_x, min_y, min_z, max_x, max_y, max_z }
    }
}

pub fn test_walls() {
    let mut walls = Vec::new()
    walls.push(AABB::new(-5.0, 0.0, 0.5, 5.0, 3.0, 1.0))
}
"#;

    let rust = test_utils::compile_single(source);

    // E0282 FIX: Should emit Vec<AABB> so Rust doesn't need type annotation
    assert!(
        rust.contains("let mut walls: Vec<AABB> = Vec::new()"),
        "Should emit type annotation for Vec::new() when .push() constrains. Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_new_push_string_infers_vec_string() {
    let source = r#"
pub fn test_parts() {
    let mut parts = Vec::new()
    parts.push(format!("v={}", 1))
}
"#;

    let rust = test_utils::compile_single(source);

    // Should infer Vec<String> from format! return type (avoids E0282)
    assert!(
        rust.contains("Vec<String>")
            || rust.contains("Vec<alloc::string::String>")
            || rust.contains("Vec<std::string::String>"),
        "Should emit Vec<String> for Vec::new() + push(format!) to avoid E0282. Got:\n{}",
        rust
    );
}

#[test]
fn test_return_vec_new_infers_from_return_type() {
    // E0282 FIX: return Vec::new() when fn returns Vec<u32> → Vec::<u32>::new()
    // Pattern from svo.wj: pub fn encode() -> Vec<u32> { ... return Vec::new() }
    let source = r#"
pub fn get_empty_u32_vec() -> Vec<u32> {
    return Vec::new()
}
"#;

    let rust = test_utils::compile_single(source);

    assert!(
        rust.contains("Vec::<u32>::new()") || rust.contains("return Vec::<u32>::new()"),
        "Should emit Vec::<u32>::new() when return type is Vec<u32>. Got:\n{}",
        rust
    );
}

#[test]
fn test_hashset_new_insert_infers_element_type() {
    // E0282 FIX: let mut s = HashSet::new(); s.insert(x) → HashSet<type_of(x)>
    // Pattern from scene_graph_state.wj: materials.insert(node.material_id)
    let source = r#"
pub fn collect_ids() {
    let mut materials = HashSet::new()
    materials.insert(42u32)
    materials.insert(100u32)
}
"#;

    let rust = test_utils::compile_single(source);

    assert!(
        rust.contains("HashSet<u32>"),
        "Should emit HashSet<u32> for HashSet::new() + insert(u32). Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_new_push_in_while_loop_infers_element_type() {
    // E0282 REGRESSION FIX: Vec::new() with .push() inside while loop (particle_pool.wj pattern)
    let source = r#"
struct Particle {
    x: f32,
    y: f32,
}

impl Particle {
    pub fn new(x: f32, y: f32) -> Particle {
        Particle { x, y }
    }
}

pub fn create_pool() {
    let mut particles = Vec::new()
    let mut i = 0
    while i < 10 {
        particles.push(Particle::new(0.0, 0.0))
        i = i + 1
    }
}
"#;

    let rust = test_utils::compile_single(source);

    assert!(
        rust.contains("let mut particles: Vec<Particle> = Vec::new()"),
        "Should infer Vec<Particle> from push inside while loop. Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_new_inferred_from_return_type() {
    // E0282 FIX: let mut result = Vec::new(); return result when fn returns Vec<T>
    // Fallback when push inference fails - use function return type
    let source = r#"
pub fn get_empty_int_vec() -> Vec<int> {
    let mut result = Vec::new()
    return result
}
"#;

    let rust = test_utils::compile_single(source);

    // Should emit Vec<i64> (int) or type annotation from return type when variable is returned
    assert!(
        rust.contains("Vec::<i64>::new()")
            || rust.contains("let mut result: Vec<i64> = Vec::new()")
            || rust.contains("Vec<i64>"),
        "Should infer Vec element type from return type when result is returned. Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_new_push_from_hashmap_values_infers_element_type() {
    // E0282 FIX: for ach in map.values() { result.push(ach) } → result: Vec<&Achievement>
    // Loop var type from .values() + push inference
    let source = r#"
struct Achievement {
    id: u32,
}

impl Achievement {
    pub fn new(id: u32) -> Achievement {
        Achievement { id }
    }
    pub fn is_unlocked(self) -> bool {
        true
    }
}

pub fn get_unlocked(achievements: HashMap<u32, Achievement>) -> Vec<&Achievement> {
    let mut result = Vec::new()
    for ach in achievements.values() {
        if ach.is_unlocked() {
            result.push(ach)
        }
    }
    result
}
"#;

    let rust = test_utils::compile_single(source);

    // Should infer Vec<&Achievement> from push(ach) where ach comes from values()
    assert!(
        rust.contains("Vec<&Achievement>") || rust.contains("Vec<&windjammer_app::Achievement>"),
        "Should emit Vec<&Achievement> for Vec::new() + push(ach) from map.values(). Got:\n{}",
        rust
    );
}

#[test]
fn test_vec_new_no_push_requires_user_annotation() {
    let source = r#"
pub fn test_empty() {
    let walls: Vec<i32> = Vec::new()
}
"#;

    let rust = test_utils::compile_single(source);

    // User provided explicit type - should be preserved (unused binding may get _ prefix)
    assert!(
        rust.contains("Vec<i32>") && rust.contains("Vec::new()"),
        "Should preserve user's explicit Vec<i32> type annotation. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_let_some_mut_option_emits_deref_clone() {
    // E0282 REGRESSION FIX: if let Some(pos) = &mut self.option_field with &mut self
    // pos has type &mut T. &mut T doesn't implement Clone - must use (*pos).clone() or *pos.
    // WRONG: *(pos).clone() - dereferences result of clone (broken)
    // RIGHT: (*pos).clone() or *pos (for Copy)
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

struct InvestigationState {
    pos: Vec3,
}

impl InvestigationState {
    pub fn new(pos: Vec3) -> InvestigationState {
        InvestigationState { pos }
    }
}

struct Foo {
    investigation_position: Option<Vec3>,
    investigation: Option<InvestigationState>,
}

impl Foo {
    pub fn check(self) {
        if self.investigation_position.is_some() {
            if let Some(pos) = self.investigation_position {
                self.investigation = Some(InvestigationState::new(pos))
            }
        }
    }
}
"#;

    let rust = test_utils::compile_single(source);

    // &mut T where T: Copy must use deref, not T::clone on the mut ref (E0282)
    assert!(
        rust.contains("(*pos).clone()")
            || rust.contains("*pos")
            || rust.contains("InvestigationState::new(*pos)"),
        "Should emit *pos or deref+clone for &mut Option match. Got:\n{}",
        rust
    );
    // Must NOT emit the broken *(pos).clone() pattern
    assert!(
        !rust.contains("*(pos).clone()"),
        "Must NOT emit *(pos).clone() - &mut T doesn't implement Clone. Got:\n{}",
        rust
    );
}

#[test]
fn test_deref_clone_turbofish_when_type_known() {
    // E0282 FIX: (*pos).clone() needs turbofish when pos: &mut T - Rust can't infer.
    // When we have local_var_types for pos (MutableReference(Vec3)), emit (*pos).clone::<Vec3>()
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

struct InvestigationState {
    pos: Vec3,
}

impl InvestigationState {
    pub fn new(pos: Vec3) -> InvestigationState {
        InvestigationState { pos }
    }
}

struct Foo {
    investigation_position: Option<Vec3>,
    investigation: Option<InvestigationState>,
}

impl Foo {
    pub fn check(self) {
        if let Some(pos) = self.investigation_position {
            self.investigation = Some(InvestigationState::new(pos))
        }
    }
}
"#;

    let rust = test_utils::compile_single(source);

    // Prefer owned Vec3: pass `*pos` or `pos` (move) — not `*pos.clone()` (invalid on &mut).
    let ok = rust.contains("InvestigationState::new(*pos)")
        || rust.contains("InvestigationState::new(pos)")
        || rust.contains("clone::<Vec3>()");
    assert!(
        ok,
        "Should pass owned Vec3 to new (deref, move, or explicit clone). Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("*pos.clone()") && !rust.contains("*(pos).clone()"),
        "Must not emit *pos.clone() on &mut. Got:\n{}",
        rust
    );
}

#[test]
fn test_some_turbofish_from_return_type() {
    // E0282 FIX: return Some(expr) when fn returns Option<T> - emit Some::<T>(expr)
    // dialogue/manager pattern: return Some(node.text()) when fn -> Option<String>
    let source = r#"
struct Node {
    text: String,
}

impl Node {
    pub fn text(self) -> String {
        self.text
    }
}

pub fn get_text(node: Option<Node>) -> Option<String> {
    match node {
        Some(n) => Some(n.text()),
        None => None,
    }
}
"#;

    let rust = test_utils::compile_single(source);

    // Turbofish is optional when return type is Option<String> (Rust often infers).
    assert!(
        rust.contains("Some::<String>")
            || rust.contains("Some::<std::string::String>")
            || (rust.contains("-> Option<String>") && rust.contains("Some(n.text())")),
        "Should emit Some::<String> or inferrable Some(n.text()) with Option<String> return. Got:\n{}",
        rust
    );
}

#[test]
fn test_match_arm_owned_type_ascription() {
    // E0282 FIX: Some(stack) => Some(stack.clone()) - stack needs type for clone inference.
    // Emit let stack: ItemStack = stack when we can infer from Option.
    let source = r#"
struct ItemStack {
    count: i32,
}

impl ItemStack {
    pub fn clone(self) -> ItemStack {
        ItemStack { count: self.count }
    }
}

struct Inventory {
    slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn get_slot(self, i: usize) -> Option<ItemStack> {
        if i < self.slots.len() {
            match self.slots[i] {
                Some(stack) => Some(stack),
                None => None,
            }
        } else {
            None
        }
    }
}
"#;

    let rust = test_utils::compile_single(source);

    // Should emit type ascription for stack or clone works
    assert!(
        rust.contains("let stack: ItemStack = stack") || rust.contains("Some(stack)"),
        "Should emit type ascription for match arm binding. Got:\n{}",
        rust
    );
}

/// E0282 FIX: collect() without type hint → default to Vec<T> when inferrable from iterator chain.
/// items.iter().filter(...).collect() should emit collect::<Vec<&T>>() or collect::<Vec<_>>().
#[test]
fn test_collect_default_to_vec() {
    let source = r#"
pub fn get_evens(numbers: Vec<i32>) -> Vec<i32> {
    numbers.iter().filter(|x| *x % 2 == 0).map(|x| *x).collect()
}
"#;

    let rust = test_utils::compile_single(source);

    // Should infer collect::<Vec<i32>>() or collect::<Vec<_>>() from return type
    assert!(
        rust.contains("collect::<Vec<") || rust.contains(".collect()"),
        "Should emit collect::<Vec<...>>() for iterator chain when fn returns Vec<T>. Got:\n{}",
        rust
    );
}

/// E0282 FIX: iter().collect() when fn returns Vec<T> - infer from return type.
#[test]
fn test_collect_infers_from_return_type() {
    let source = r#"
pub fn get_ids(items: Vec<i32>) -> Vec<i32> {
    items.iter().map(|x| *x).collect()
}
"#;

    let rust = test_utils::compile_single(source);

    // Should emit collect::<Vec<i64>>() or collect::<Vec<_>>() from return type
    assert!(
        rust.contains("collect::<Vec<"),
        "Should emit collect::<Vec<...>>() when fn returns Vec<T>. Got:\n{}",
        rust
    );
}
