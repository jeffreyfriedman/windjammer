use crate::test_utils::compile_single_check;

// ============================================================================
// RC1: for-loop over self.field must borrow when self is &self
//
// Pattern from: visual_verification.wj, shader_graph_builder.wj, voxel_scene.wj
// Error: E0382 — use of moved value / borrow of partially moved value
// Root cause: `for p in self.pixels` consumes self.pixels, then self.pixels.len()
//             fails because the field was moved into the for loop.
// Fix: compiler should emit `for p in &self.pixels` when method has `&self`.
// ============================================================================

#[test]
fn test_for_loop_self_field_then_use_field_after() {
    let source = r#"
struct Pixel {
    r: f32
    g: f32
    b: f32
}

struct Image {
    pixels: Vec<Pixel>
}

impl Image {
    pub fn brightness(self) -> f32 {
        let mut sum = 0.0
        for p in self.pixels {
            sum = sum + p.r + p.g + p.b
        }
        let total = self.pixels.len()
        if total == 0 {
            return 0.0
        }
        sum / (total as f32)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        rust_code.contains("for p in &self.pixels")
            || rust_code.contains("for p in & self.pixels"),
        "for-loop over self.pixels should borrow: got:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("fn brightness(self)"),
        "brightness should not take owned self — should be &self"
    );
}

#[test]
fn test_for_loop_self_field_read_only_method_on_element() {
    let source = r#"
struct Item {
    name: string
    active: bool
}

impl Item {
    pub fn is_valid(self) -> bool {
        self.active
    }
}

struct Container {
    items: Vec<Item>
}

impl Container {
    pub fn count_valid(self) -> i32 {
        let mut count = 0
        for item in self.items {
            if item.is_valid() {
                count = count + 1
            }
        }
        let total = self.items.len()
        if total == 0 { return 0 }
        count
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    // Method uses self.items.len() after the for-loop, so either:
    // 1. self is &self and for-loop borrows, OR
    // 2. for-loop must borrow even with owned self to avoid consuming the Vec
    assert!(
        rust_code.contains("for item in &self.items")
            || rust_code.contains("for item in & self.items"),
        "for-loop should borrow self.items (field used after loop), got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC2: Read-only methods should get &self, not owned self
//
// Pattern from: camera2d.wj (is_shaking, shake_intensity, shake_offset)
// Error: E0382/E0507 — calling owned-self method from &mut self context
// Root cause: Analyzer infers Owned for methods that only compare fields.
// ============================================================================

#[test]
fn test_readonly_method_called_from_mut_method() {
    // Non-Copy struct (has String field) — methods that only read fields
    // should get &self, not owned self. Otherwise calling from &mut self fails.
    let source = r#"
struct Camera {
    name: string
    shake_timer: f32
    shake_duration: f32
    shake_intensity: f32
    offset_x: f32
    offset_y: f32
}

impl Camera {
    pub fn is_shaking(self) -> bool {
        self.shake_timer < self.shake_duration
    }

    pub fn get_intensity(self) -> f32 {
        if self.is_shaking() {
            let progress = self.shake_timer / self.shake_duration
            self.shake_intensity * (1.0 - progress)
        } else {
            0.0
        }
    }

    pub fn update(self, delta: f32) {
        if self.is_shaking() {
            self.shake_timer = self.shake_timer + delta
            if self.shake_timer >= self.shake_duration {
                self.offset_x = 0.0
                self.offset_y = 0.0
            } else {
                let intensity = self.get_intensity()
                self.offset_x = intensity * 0.5
            }
        }
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        rust_code.contains("fn is_shaking(&self)") || rust_code.contains("fn is_shaking(& self)"),
        "is_shaking should be &self (read-only on non-Copy), got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("fn get_intensity(&self)")
            || rust_code.contains("fn get_intensity(& self)"),
        "get_intensity should be &self (read-only on non-Copy), got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC2b: Read-only method on field called from &self method
//
// Pattern from: visual_verification.wj (self.screenshot.has_color(),
//   self.screenshot.is_solid_color())
// Error: E0507 — cannot move out of self.screenshot behind shared reference
// Root cause: has_color(self) / is_solid_color(self) take owned self but are
//   called on self.screenshot which is behind &self.
// ============================================================================

#[test]
fn test_readonly_field_method_called_from_ref_self() {
    let source = r#"
struct Color {
    r: f32
    g: f32
    b: f32
}

struct Snapshot {
    pixels: Vec<Color>
}

impl Snapshot {
    pub fn is_solid(self, tolerance: f32) -> bool {
        if self.pixels.len() == 0 {
            return true
        }
        true
    }
}

struct Verifier {
    screenshot: Snapshot
}

impl Verifier {
    pub fn check_not_solid(self) -> bool {
        !self.screenshot.is_solid(0.01)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        rust_code.contains("fn is_solid(&self"),
        "is_solid should be &self, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC2c: Builder method that calls self.method() then reconstructs struct
//
// Pattern from: scene/builder.wj (self.calculate_bounds() takes owned self)
// Error: E0382 — use of moved value: self.materials
// Root cause: calculate_bounds consumes self, then return expression uses self.fields
// ============================================================================

#[test]
fn test_builder_calls_readonly_method_then_uses_fields() {
    let source = r#"
struct Bounds {
    min_x: f32
    max_x: f32
}

struct Builder {
    grids: Vec<f32>
    cameras: Vec<f32>
    materials: Vec<f32>
}

impl Builder {
    pub fn calculate_bounds(self) -> Bounds {
        Bounds { min_x: 0.0, max_x: 1.0 }
    }

    pub fn auto_frame(self) -> Builder {
        let bounds = self.calculate_bounds()
        Builder { grids: self.grids, cameras: self.cameras, materials: self.materials }
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        rust_code.contains("fn calculate_bounds(&self")
            || rust_code.contains("fn calculate_bounds(& self"),
        "calculate_bounds should be &self (returns computed value, doesn't consume fields), got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC3: Missing auto-borrow for function arguments
//
// Pattern from: physics_world.wj (check_collision(body_a, body_b) needs &)
// Error: E0308 — expected &RigidBody2D, found RigidBody2D
// ============================================================================

#[test]
fn test_auto_borrow_for_function_args() {
    // Non-Copy struct — function reads fields only, so params should be & borrowed.
    // At call site, owned values should be auto-borrowed.
    let source = r#"
struct Body {
    name: string
    x: f32
    y: f32
}

struct Collision {
    depth: f32
}

fn check_collision(a: Body, b: Body) -> Option<Collision> {
    let dx = a.x - b.x
    if dx < 1.0 {
        Some(Collision { depth: dx })
    } else {
        None
    }
}

struct World {
    bodies: Vec<Body>
}

impl World {
    pub fn step(self) -> Vec<Collision> {
        let mut events = Vec::new()
        let a = self.bodies[0]
        let b = self.bodies[1]
        match check_collision(a, b) {
            Some(c) => events.push(c),
            None => {}
        }
        events
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    // check_collision takes read-only params on non-Copy type, should get &Body
    assert!(
        rust_code.contains("fn check_collision(a: &Body, b: &Body)")
            || rust_code.contains("check_collision(&a, &b)")
            || rust_code.contains("check_collision(& a, & b)"),
        "check_collision should auto-borrow non-Copy args/params, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC3b: HashMap::remove needs &key
//
// Pattern from: voxel_world.wj, timer/manager.wj
// Error: E0308 — expected &(i32,i32,i32), found (i32,i32,i32)
// ============================================================================

#[test]
fn test_hashmap_remove_auto_borrows_key() {
    let source = r#"
use std::collections::HashMap

struct TimerId {
    id: i32
}

struct Timer {
    elapsed: f32
}

struct TimerManager {
    timers: HashMap<TimerId, Timer>
}

impl TimerManager {
    pub fn remove_timer(self, id: TimerId) {
        self.timers.remove(id)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        rust_code.contains("self.timers.remove(&id)"),
        "HashMap::remove should auto-borrow key, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC4: Over-referencing — &mut where owned expected
//
// Pattern from: voxel_editor.wj, game_renderer.wj
// Error: E0308 — expected CameraData, found &mut CameraData
// ============================================================================

#[test]
fn test_no_extra_ref_for_owned_param() {
    let source = r#"
struct CameraData {
    fov: f32
    near: f32
    far: f32
}

struct Renderer {
    active: bool
}

impl Renderer {
    pub fn set_camera(self, camera: CameraData) {
        self.active = true
    }
}

struct Editor {
    renderer: Renderer
}

impl Editor {
    pub fn update_camera(self) {
        let camera = CameraData { fov: 60.0, near: 0.1, far: 100.0 }
        self.renderer.set_camera(camera)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    // Should NOT have &mut camera when passing to set_camera
    assert!(
        !rust_code.contains("set_camera(&mut camera)"),
        "Should not over-reference camera arg, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC4b: Over-referencing — &mut on Copy-type args via trait impl
//
// Pattern from: voxel_editor.wj → self.renderer.set_camera(camera)
// Error: E0308 — expected CameraData, found &mut CameraData
// Root cause: The trait impl method has &mut self for the receiver, and
//             the compiler incorrectly extends &mut to the Copy-type data arg.
// ============================================================================

#[test]
fn test_no_extra_ref_for_copy_param_in_trait_impl() {
    let source = r#"
struct CameraData {
    fov: f32
    near: f32
    far: f32
}

trait RenderPort {
    fn set_camera(camera: CameraData)
}

struct VoxelRenderer {
    active: bool
}

impl RenderPort for VoxelRenderer {
    fn set_camera(camera: CameraData) {
        self.active = true
    }
}

struct Editor {
    renderer: VoxelRenderer
}

impl Editor {
    pub fn update_camera(self) {
        let camera = CameraData { fov: 60.0, near: 0.1, far: 100.0 }
        self.renderer.set_camera(camera)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        !rust_code.contains("set_camera(&mut camera)"),
        "Should not over-reference Copy-type camera arg in trait impl call, got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("set_camera(camera)"),
        "Should pass Copy type by value, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC5: Mutability inference — self should be &mut self
//
// Pattern from: physics_world.wj (step), shader_graph_builder.wj (build)
// Error: E0596 — cannot borrow self.bodies as mutable, self not declared mutable
// ============================================================================

#[test]
fn test_mut_self_when_body_mutates_field() {
    let source = r#"
struct Graph {
    passes: Vec<i32>
    resolved: bool
}

impl Graph {
    pub fn resolve_dependencies(self) {
        self.resolved = true
    }

    pub fn build(self) -> Vec<i32> {
        self.resolve_dependencies()
        self.passes
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    // build() calls resolve_dependencies which mutates self, so build needs &mut self
    // OR build consumes self (returns self.passes)
    // Either way, build should not be plain `self` without mut
    let has_correct_self = rust_code.contains("fn build(&mut self")
        || rust_code.contains("fn build(mut self");
    assert!(
        has_correct_self,
        "build should be &mut self or mut self since it mutates and returns field, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC6: Spurious deref on tuple in for-loop
//
// Pattern from: voxel_world.wj — dirty.push(*pos) where pos is (i32,i32,i32)
// Error: E0614 — type (i32,i32,i32) cannot be dereferenced
// ============================================================================

#[test]
fn test_no_spurious_deref_on_for_loop_tuple() {
    let source = r#"
use std::collections::HashMap

struct Chunk {
    data: Vec<i32>
}

struct World {
    chunks: HashMap<(i32, i32, i32), Chunk>
}

impl World {
    pub fn get_dirty(self) -> Vec<(i32, i32, i32)> {
        let mut dirty = Vec::new()
        for (pos, chunk) in self.chunks {
            if chunk.data.len() > 0 {
                dirty.push(pos)
            }
        }
        dirty
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        !rust_code.contains("*pos"),
        "Should not deref tuple pos, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC7: Bool behind reference not auto-derefed
//
// Pattern from: save/data.wj — `if v {` where v is &bool from HashMap iteration
// Error: E0308 — expected bool, found &bool
// ============================================================================

#[test]
fn test_bool_ref_auto_deref_in_condition() {
    let source = r#"
use std::collections::HashMap

struct SaveData {
    flags: HashMap<string, bool>
}

impl SaveData {
    pub fn count_true(self) -> i32 {
        let mut count = 0
        for (key, val) in self.flags {
            if val {
                count = count + 1
            }
        }
        count
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    // When iterating &self.flags (HashMap), val is &bool.
    // The condition must be `if *val` to auto-deref the borrowed bool.
    assert!(
        rust_code.contains("if *val"),
        "Should auto-deref borrowed bool: expected `if *val`, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC7b: Bool deref in if-else expression (save/data.wj serialize pattern)
//
// Pattern: `let val = if v { "true" } else { "false" }` inside HashMap iteration
// Error: E0308 — expected bool, found &bool
// ============================================================================

#[test]
fn test_bool_ref_auto_deref_if_else_expression() {
    let source = r#"
use std::collections::HashMap

struct SaveData {
    bool_fields: HashMap<string, bool>
}

impl SaveData {
    pub fn serialize(self) -> string {
        let mut out = ""
        for (k, v) in self.bool_fields {
            let val = if v { "true" } else { "false" }
            out.push_str(val)
        }
        out
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        rust_code.contains("if *v"),
        "Should auto-deref borrowed bool in if-else expression: expected `if *v`, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC8: Vec::remove takes usize, not &usize
//
// Pattern from: text_input.wj — codepoints.remove(&pos)
// Error: E0308 — expected usize, found &usize
// ============================================================================

#[test]
fn test_vec_remove_no_borrow_on_index() {
    let source = r#"
struct TextInput {
    codepoints: Vec<i32>
    cursor: i32
}

impl TextInput {
    pub fn delete_char(self) {
        let pos = self.cursor as usize
        self.codepoints.remove(pos)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        !rust_code.contains("remove(&pos)"),
        "Vec::remove should NOT borrow usize index, got:\n{}",
        rust_code
    );
}

// ============================================================================
// RC8b: Vec::remove on local variable with explicit usize type annotation
//
// Pattern from: text_input.wj — `let pos: usize = ...; codepoints.remove(pos)`
// Where `codepoints` is a local Vec (not a struct field), so the receiver type
// may not be inferred as Vec. The Copy-type detection must work for local vars.
// ============================================================================

#[test]
fn test_vec_remove_local_var_usize_annotation() {
    let source = r#"
struct TextInput {
    text: string
    cursor: i32
}

impl TextInput {
    pub fn delete_char(self) {
        let pos: usize = (self.cursor - 1) as usize
        let mut codepoints = Vec::new()
        codepoints.push(1)
        codepoints.remove(pos)
    }
}
"#;
    let (rust_code, ok) = compile_single_check(source);
    assert!(ok, "Compilation failed");
    assert!(
        !rust_code.contains("remove(&pos)"),
        "Vec::remove should not borrow index, got:\n{}",
        rust_code
    );
}
