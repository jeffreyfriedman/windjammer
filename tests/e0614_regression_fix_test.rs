//! TDD Test: E0614 "cannot be dereferenced" regression fix
//!
//! Phase 7 fix was incomplete - expression_is_reference() returned true for:
//! - self.field when field is Copy (i32, f32, u32) - yields T, not &T
//! - var.field (world.world_size, item.value) - same
//!
//! Root cause: FieldAccess recursed to object; for self.field, object=self is &self,
//! so we wrongly returned true and added *(self.x) when self.x is i32.
//!
//! Fix: For FieldAccess, check the FIELD's type via infer_expression_type(expr).
//! If Copy, return false (expression yields T by value).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_e0614_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

// === Pattern 1: self.field with Copy fields (f32, i32, u32) ===

#[test]
fn test_self_current_state_i32_no_deref() {
    // ai/state_machine: transition.matches(*(self.current_state)) - current_state is i32
    let source = r#"
pub fn matches(state: int) -> bool {
    true
}

pub struct StateMachine {
    pub current_state: int,
}

impl StateMachine {
    pub fn check(self, to_state: int) -> bool {
        matches(self.current_state) && to_state == 0
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.current_state)"),
        "Should NOT add * to self.current_state (i32). Generated:\n{}",
        rs
    );
}

#[test]
fn test_self_x_y_i32_no_deref() {
    // ai/pathfinding: grid.grid_to_world(*(self.x), *(self.y))
    let source = r#"
pub fn grid_to_world(x: int, y: int) -> (f32, f32) {
    (0.0, 0.0)
}

pub struct PathNode {
    pub x: int,
    pub y: int,
}

impl PathNode {
    pub fn to_world(self, grid: bool) -> (f32, f32) {
        grid_to_world(self.x, self.y)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.x)") && !rs.contains("*(self.y)"),
        "Should NOT add * to self.x, self.y (i32). Generated:\n{}",
        rs
    );
}

#[test]
fn test_self_root_id_i32_no_deref() {
    // csg/scene: self.emit_node_instructions(*(self.root_id), ...)
    let source = r#"
pub fn emit_node_instructions(node_id: int, buf: Vec<f32>) {
}

pub struct Scene {
    pub root_id: int,
}

impl Scene {
    pub fn emit(self, buf: Vec<f32>) {
        emit_node_instructions(self.root_id, buf)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.root_id)"),
        "Should NOT add * to self.root_id (i32). Generated:\n{}",
        rs
    );
}

#[test]
fn test_self_current_time_f32_no_deref() {
    // editor/animation_editor: evaluate(*(self.current_time))
    let source = r#"
pub fn evaluate(t: f32) -> f32 {
    t
}

pub struct Track {
    pub current_time: f32,
}

impl Track {
    pub fn eval(self) -> f32 {
        evaluate(self.current_time)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.current_time)"),
        "Should NOT add * to self.current_time (f32). Generated:\n{}",
        rs
    );
}

#[test]
fn test_quat_self_fields_f32_no_deref() {
    // math/quat: Vec3::new(*(self.x), *(self.y), *(self.z))
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub fn vector_part(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.x)") && !rs.contains("*(self.y)") && !rs.contains("*(self.z)"),
        "Should NOT add * to Quat f32 fields. Generated:\n{}",
        rs
    );
}

#[test]
fn test_camera_orthographic_f32_no_deref() {
    // rendering3d/camera3d: Mat4::orthographic(*(self.left), *(self.right), ...)
    let source = r#"
pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> bool {
    true
}

pub struct Camera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn ortho(self) -> bool {
        orthographic(self.left, self.right, self.bottom, self.top, self.near, self.far)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.left)") && !rs.contains("*(self.right)"),
        "Should NOT add * to camera f32 fields. Generated:\n{}",
        rs
    );
}

#[test]
fn test_camera_perspective_f32_no_deref() {
    let source = r#"
pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> bool {
    true
}

pub struct Camera {
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn persp(self, fov: f32) -> bool {
        perspective(fov, self.aspect, self.near, self.far)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.aspect)") && !rs.contains("*(self.near)") && !rs.contains("*(self.far)"),
        "Should NOT add * to camera f32 fields. Generated:\n{}",
        rs
    );
}

#[test]
fn test_self_relationship_value_i32_no_deref() {
    // narrative/character: RelationshipStatus::from_value(*(self.relationship_value))
    let source = r#"
pub fn from_value(v: int) -> bool {
    true
}

pub struct Character {
    pub relationship_value: int,
}

impl Character {
    pub fn status(self) -> bool {
        from_value(self.relationship_value)
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(self.relationship_value)"),
        "Should NOT add * to self.relationship_value (i32). Generated:\n{}",
        rs
    );
}

// === Pattern 2: var.field (non-self) with Copy fields ===

#[test]
fn test_world_field_u32_f32_no_deref() {
    // rendering/voxel_gpu_renderer: upload_svo(..., *(world.world_size), *(world.depth))
    let source = r#"
pub fn upload_svo(nodes: bool, world_size: u32, depth: u32) {
}

pub struct World {
    pub world_size: u32,
    pub depth: u32,
}

pub fn test(world: World) {
    upload_svo(true, world.world_size, world.depth)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(world.world_size)") && !rs.contains("*(world.depth)"),
        "Should NOT add * to world fields. Generated:\n{}",
        rs
    );
}

#[test]
fn test_item_value_i32_no_deref() {
    // rpg/trading: modifiers.calculate_buy_price(*(item.value), ...)
    let source = r#"
pub fn calculate_buy_price(value: int, t: bool) -> int {
    value
}

pub struct Item {
    pub value: int,
}

pub fn test(item: Item, modifiers: bool) -> int {
    calculate_buy_price(item.value, modifiers)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(item.value)"),
        "Should NOT add * to item.value (i32). Generated:\n{}",
        rs
    );
}

#[test]
fn test_stack_quantity_i32_no_deref() {
    // rpg/trading: merchant.has_item(..., *(stack.quantity.clone()))
    let source = r#"
pub fn has_item(id: bool, qty: int) -> bool {
    true
}

pub struct Stack {
    pub quantity: int,
}

pub fn test(merchant: bool, stack: Stack) -> bool {
    has_item(merchant, stack.quantity)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(stack.quantity)"),
        "Should NOT add * to stack.quantity (i32). Generated:\n{}",
        rs
    );
}

// === Pattern 3: Literals (existing tests) ===

#[test]
fn test_f32_literal_no_deref() {
    let source = r#"
pub fn take_float(x: f32) -> f32 {
    x
}

pub fn test() -> f32 {
    take_float(1.0)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(!rs.contains("*(1.0") && !rs.contains("*(1.0_f32)"));
}

#[test]
fn test_i32_literal_no_deref() {
    let source = r#"
pub fn take_int(x: int) -> int {
    x
}

pub fn test() -> int {
    take_int(42)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(!rs.contains("*(42)"));
}

// === Pattern 4: Array/Vec index with Copy (existing) ===

#[test]
fn test_vec4_from_array_no_deref() {
    let source = r#"
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
        Vec4 { x, y, z, w }
    }

    pub fn from_array(arr: [f32; 4]) -> Vec4 {
        Vec4::new(arr[0], arr[1], arr[2], arr[3])
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(arr[0])") && !rs.contains("*(arr[1])"),
        "Should NOT add * to array index. Generated:\n{}",
        rs
    );
}

// === Pattern 5: Method call .clone() returns owned ===

#[test]
fn test_clone_returns_owned_no_deref() {
    // item_id.clone() returns u32, not &u32
    let source = r#"
pub fn take_u32(x: u32) -> u32 {
    x
}

pub fn test(item_id: u32) -> u32 {
    take_u32(item_id)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

// === Pattern 6: Match arm + .clone() - Phase 9 (dialogue/system pattern) ===

#[test]
fn test_match_arm_clone_u32_no_deref() {
    // DialogueConsequence::GiveItem(item_id) => state.give_item(item_id.clone())
    // item_id.clone() returns u32, NOT &u32 - must NOT add *
    let source = r#"
pub fn give_item(id: u32) {
}

pub enum Consequence {
    GiveItem(u32),
}

pub fn apply(self, state: bool) {
    match self {
        Consequence::GiveItem(item_id) => {
            give_item(item_id.clone())
        },
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(item_id") && !rs.contains("*(item_id.clone())"),
        "Should NOT add * to item_id.clone() (returns u32). Generated:\n{}",
        rs
    );
}

// === Pattern 7: Loop variable - owned iterator (ecs/world pattern) ===

#[test]
fn test_loop_entity_owned_no_deref() {
    // for x in vec { result.push(x) } - vec yields owned, must NOT add *
    let source = r#"
pub fn query() -> Vec<int> {
    let mut result = Vec::new()
    let nums = vec![1, 2, 3]
    for x in nums {
        result.push(x)
    }
    result
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(x)"),
        "Should NOT add * when iterator yields int by value. Generated:\n{}",
        rs
    );
}

// === Pattern 8: FieldAccess on param - world.world_size (voxel_gpu_renderer) ===

#[test]
fn test_param_field_world_size_no_deref() {
    // upload_svo(..., world.world_size, world.depth) - world is param
    let source = r#"
pub fn upload_svo(nodes: bool, world_size: f32, depth: u32) {
}

pub struct VoxelWorld {
    pub world_size: f32,
    pub depth: u32,
}

pub fn render(world: VoxelWorld) {
    upload_svo(true, world.world_size, world.depth)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(world.world_size)") && !rs.contains("*(world.depth)"),
        "Should NOT add * to world fields. Generated:\n{}",
        rs
    );
}

// === Pattern 9: MethodCall .clone() on FieldAccess (shader_graph_executor) ===

#[test]
fn test_pass_groups_clone_no_deref() {
    // for pass in self.passes { execute_pass(pass, pass.groups_x.clone(), ...) }
    let source = r#"
pub fn execute_pass(pass: bool, x: u32, y: u32, z: u32) {
}

pub struct Pass {
    pub groups_x: u32,
    pub groups_y: u32,
    pub groups_z: u32,
}

pub fn run(passes: Vec<Pass>) {
    for pass in passes {
        execute_pass(true, pass.groups_x.clone(), pass.groups_y.clone(), pass.groups_z.clone())
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(pass.groups_x") && !rs.contains("*(pass.groups_y") && !rs.contains("*(pass.groups_z"),
        "Should NOT add * to pass.groups_*.clone() (returns u32). Generated:\n{}",
        rs
    );
}
