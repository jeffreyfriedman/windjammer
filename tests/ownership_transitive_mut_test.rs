//! TDD Test: Transitive mutability inference for ownership
//!
//! Bug: E0596 - cannot borrow as mutable (15 errors in windjammer-game)
//! Root Cause: Ownership inference doesn't track transitivity:
//! - If self.field.method() needs &mut field → self needs &mut self
//! - If for item in self.vec { item.mutate() } → self needs &mut self
//! - If self.channels[i].mute() → self needs &mut self
//!
//! Fix: Extend ownership inference to detect transitive mutation through fields.
//! Philosophy: "Safety Without Ceremony" - infer ownership correctly.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    let wj_binary = if wj_binary.exists() {
        wj_binary
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/wj")
    };

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = test_dir.join("build/test.rs");
    let generated =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");
    Ok(generated)
}

fn verify_rust_compiles(rust_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = temp_dir.path().join("test.rs");
    std::fs::write(&rust_file, rust_code).expect("Failed to write Rust file");

    let check = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            rust_file.to_str().unwrap(),
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if check.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&check.stderr).to_string())
    }
}

#[test]
fn test_self_field_mutating_method() {
    // self.field.mutating_method() → &mut self
    let code = r#"
pub struct Channel {
    pub muted: bool,
}

impl Channel {
    pub fn mute(self) {
        self.muted = true
    }
}

pub struct Mixer {
    pub channels: Vec<Channel>,
}

impl Mixer {
    pub fn mute_channel(self, id: i32) {
        self.channels[id as usize].mute()
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn mute_channel(&mut self"),
        "mute_channel should infer &mut self from self.channels[i].mute(). Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_for_loop_mutating_elements() {
    // for squad in self.squads { squad.clear_old_messages(...) } → &mut self
    let code = r#"
pub struct Message {
    pub text: string,
}

pub struct Squad {
    pub id: i32,
    pub messages: Vec<Message>,
}

impl Squad {
    pub fn clear_old_messages(self, _current_time: f32, _max_age: f32) {
        self.messages.clear()
    }
}

pub struct Tactics {
    pub squads: Vec<Squad>,
}

impl Tactics {
    pub fn cleanup(self, current_time: f32) {
        for squad in self.squads {
            squad.clear_old_messages(current_time, 10.0)
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn cleanup(&mut self"),
        "cleanup should infer &mut self from for-loop mutating elements. Generated:\n{}",
        rust
    );
    // Must use &mut self.squads in the for loop
    assert!(
        rust.contains("&mut self.squads") || rust.contains("for squad in &mut self.squads"),
        "For loop should iterate &mut self.squads when mutating elements. Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_indexed_field_mute() {
    // self.channels[i].mute() → &mut self (exact pattern from audio/mixer.rs)
    let code = r#"
pub struct AudioChannel {
    pub volume: f32,
}

impl AudioChannel {
    pub fn mute(self) {
        let _ = self.volume;
        self.volume = 0.0
    }
}

pub struct Mixer {
    pub channels: Vec<AudioChannel>,
}

impl Mixer {
    pub fn mute_channel(self, channel_id: i32) {
        self.channels[channel_id as usize].mute()
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn mute_channel(&mut self"),
        "mute_channel should infer &mut self. Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_nested_field_update_camera() {
    // self.renderer.update_camera() → &mut self (pattern from demos/cathedral.rs)
    let code = r#"
pub struct Camera {
    pub x: f32,
}

pub struct Renderer {
    pub camera: Camera,
}

impl Renderer {
    pub fn update_camera(self, _cam: Camera) {
        let _ = self.camera
    }
    pub fn render_frame(self) {
        let _ = self.camera
    }
}

pub struct Demo {
    pub renderer: Renderer,
}

impl Demo {
    pub fn render(self, camera: Camera) {
        self.renderer.update_camera(camera);
        self.renderer.render_frame()
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn render(&mut self"),
        "render should infer &mut self from self.renderer.update_camera(). Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_for_loop_send_message() {
    // for squad in self.squads { squad.send_message(...) } → &mut self
    let code = r#"
pub struct Squad {
    pub id: i32,
    pub messages: Vec<string>,
}

impl Squad {
    pub fn send_message(self, msg: string) {
        self.messages.push(msg)
    }
}

pub struct Tactics {
    pub squads: Vec<Squad>,
}

impl Tactics {
    pub fn broadcast(self, msg: string, sender_id: i32) {
        for squad in self.squads {
            if squad.id != sender_id {
                squad.send_message(msg.clone())
            }
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn broadcast(&mut self"),
        "broadcast should infer &mut self. Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_readonly_for_loop_no_mut() {
    // False positive prevention: for x in self.items { x.len() } → &self (read-only)
    let code = r#"
pub struct Container {
    pub items: Vec<Vec<i32>>,
}

impl Container {
    pub fn total_len(self) -> i32 {
        let mut sum = 0;
        for x in self.items {
            sum = sum + x.len()
        }
        sum
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn total_len(&self)") || rust.contains("total_len(&self"),
        "total_len should infer &self (read-only loop). Generated:\n{}",
        rust
    );
}

#[test]
fn test_direct_field_assignment() {
    // self.count = x → &mut self (baseline)
    let code = r#"
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn set(self, value: i32) {
        self.count = value
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn set(&mut self"),
        "set should infer &mut self. Generated:\n{}",
        rust
    );
}

#[test]
fn test_vec_push_on_field() {
    // self.items.push(x) → &mut self (baseline)
    let code = r#"
pub struct List {
    pub items: Vec<i32>,
}

impl List {
    pub fn add(self, item: i32) {
        self.items.push(item)
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn add(&mut self"),
        "add should infer &mut self. Generated:\n{}",
        rust
    );
}

#[test]
fn test_self_field_update_wait() {
    // self.patrol.update_wait(dt) → &mut self (E0596 fix - npc_behavior)
    let code = r#"
pub struct PatrolConfig {
    pub wait_time: f32,
}

impl PatrolConfig {
    pub fn update_wait(self, dt: f32) -> bool {
        self.wait_time = self.wait_time - dt
        self.wait_time <= 0.0
    }
}

pub struct NPCAI {
    pub patrol: PatrolConfig,
}

impl NPCAI {
    pub fn update_patrol(self, dt: f32) {
        if !self.patrol.update_wait(dt) {
            let _ = 0
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn update_patrol(&mut self"),
        "update_patrol should infer &mut self from self.patrol.update_wait(). Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_option_pattern_mut_binding() {
    // E0596 fix: if let Some(inv) = opt { inv.update(dt) } → emit if let Some(mut inv)
    // Use local Option to avoid E0507 move-from-self
    let code = r#"
pub struct InvestigationState {
    pub progress: f32,
}

impl InvestigationState {
    pub fn update(self, dt: f32) -> bool {
        self.progress = self.progress + dt
        self.progress >= 1.0
    }
    pub fn is_complete(self) -> bool {
        self.progress >= 1.0
    }
}

pub fn process_opt(opt: Option<InvestigationState>, dt: f32) {
    if let Some(inv) = opt {
        if !inv.update(dt) || inv.is_complete() {
            let _ = 0
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("if let Some(mut inv)"),
        "Option pattern should emit mut when body mutates binding. Generated:\n{}",
        rust
    );
}

#[test]
fn test_self_field_indexed_method_start() {
    // self.active_quests[i].start() → &mut self (E0596 fix - quest.rs)
    let code = r#"
pub struct Quest {
    pub started: bool,
}

impl Quest {
    pub fn start(self) -> bool {
        self.started = true
        true
    }
}

pub struct QuestLog {
    pub active_quests: Vec<Quest>,
}

impl QuestLog {
    pub fn start_quest(self, quest_id: i32) -> bool {
        let i = 0
        if i < self.active_quests.len() {
            return self.active_quests[i].start()
        }
        false
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn start_quest(&mut self"),
        "start_quest should infer &mut self from self.active_quests[i].start(). Generated:\n{}",
        rust
    );
}

#[test]
fn test_three_level_nesting() {
    // self.renderer.pipeline.shader.reload() → &mut self (Pattern 1: three-level nesting)
    let code = r#"
pub struct Shader {
    pub loaded: bool,
}

impl Shader {
    pub fn reload(self) {
        self.loaded = true
    }
}

pub struct Pipeline {
    pub shader: Shader,
}

impl Pipeline {
    pub fn get_shader(self) -> Shader {
        self.shader
    }
}

pub struct Renderer {
    pub pipeline: Pipeline,
}

pub struct Game {
    pub renderer: Renderer,
}

impl Game {
    pub fn tick(self) {
        self.renderer.pipeline.shader.reload()
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn tick(&mut self"),
        "tick should infer &mut self from self.renderer.pipeline.shader.reload(). Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_match_arm_self_field_mutation() {
    // Pattern 4: match arm mutations - match choice { 0 => self.data.field = 42 }
    let code = r#"
pub struct Data {
    pub field: i32,
}

pub struct State {
    pub data: Data,
}

impl State {
    pub fn process(self, choice: i32) {
        match choice {
            0 => self.data.field = 42,
            1 => self.data.field = 100,
            _ => {}
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn process(&mut self"),
        "process should infer &mut self from match arm field assignment. Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_for_loop_assignment_to_loop_var() {
    // E0596 fix: for entity in self.entities { entity.transform.x = 1 } → &mut self
    // Pattern: assignment to loop var fields requires &mut self (query_system.rs)
    let code = r#"
pub struct Transform {
    pub x: f32,
    pub y: f32,
}

pub struct Entity {
    pub transform: Transform,
}

pub struct QuerySystem {
    pub entities: Vec<Entity>,
}

impl QuerySystem {
    pub fn update_positions(self) {
        for entity in self.entities {
            entity.transform.x = entity.transform.x + 1.0
            entity.transform.y = entity.transform.y + 1.0
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn update_positions(&mut self"),
        "update_positions should infer &mut self from for-loop assigning to entity fields. Generated:\n{}",
        rust
    );
    assert!(
        rust.contains("&mut self.entities") || rust.contains("for entity in &mut self.entities"),
        "For loop should iterate &mut self.entities when assigning to elements. Generated:\n{}",
        rust
    );
}

#[test]
fn test_param_mutating_method_direct() {
    // Sanity check: mesh.add_quad() directly (no match) → mesh: &mut Mesh
    let code = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
pub struct Vertex { pub pos: Vec3 }
pub struct Mesh { pub vertices: Vec<Vertex> }

impl Mesh {
    pub fn new() -> Mesh { Mesh { vertices: Vec::new() } }
    pub fn add_quad(self, v0: Vertex, v1: Vertex, v2: Vertex, v3: Vertex) {
        self.vertices.push(v0)
    }
}

fn process_mesh(mesh: Mesh) {
    let v0 = Vertex { pos: Vec3 { x: 0.0, y: 0.0, z: 0.0 } }
    let v1 = Vertex { pos: Vec3 { x: 1.0, y: 0.0, z: 0.0 } }
    mesh.add_quad(v0, v1, v0, v1)
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("mesh: &mut Mesh") || rust.contains("mesh: & mut Mesh"),
        "mesh param should infer &mut Mesh when add_quad called. Generated:\n{}",
        rust
    );
}

#[test]
fn test_param_mutating_method_in_match_arm() {
    // E0596 fix: mesh.add_quad() inside match arms → mesh: &mut Mesh (mesh_generator.rs)
    let code = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
pub struct Vertex { pub pos: Vec3 }
pub struct Color { pub r: f32, pub g: f32, pub b: f32 }
pub struct Mesh { pub vertices: Vec<Vertex> }
pub enum FaceDirection { PosX, NegX }

impl Mesh {
    pub fn new() -> Mesh { Mesh { vertices: Vec::new() } }
    pub fn add_quad(self, v0: Vertex, v1: Vertex, v2: Vertex, v3: Vertex) {
        self.vertices.push(v0)
        self.vertices.push(v1)
        self.vertices.push(v2)
        self.vertices.push(v3)
    }
}

fn generate_merged_quad_x(mesh: Mesh, x: i32, direction: FaceDirection) {
    match direction {
        FaceDirection::PosX => {
            let v0 = Vertex { pos: Vec3 { x: 0.0, y: 0.0, z: 0.0 } }
            let v1 = Vertex { pos: Vec3 { x: 1.0, y: 0.0, z: 0.0 } }
            let v2 = Vertex { pos: Vec3 { x: 1.0, y: 1.0, z: 0.0 } }
            let v3 = Vertex { pos: Vec3 { x: 0.0, y: 1.0, z: 0.0 } }
            mesh.add_quad(v0, v1, v2, v3)
        },
        FaceDirection::NegX => {},
    }
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("mesh: &mut Mesh") || rust.contains("mesh: & mut Mesh"),
        "mesh param should infer &mut Mesh when add_quad is called in match arm. Generated:\n{}",
        rust
    );
    verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_self_field_reload() {
    // self.scripts[i].reload() → &mut self (E0596 fix - hot_reload.rs)
    let code = r#"
pub struct Script {
    pub loaded: bool,
}

impl Script {
    pub fn reload(self) {
        self.loaded = true
    }
}

pub struct HotReloader {
    pub scripts: Vec<Script>,
}

impl HotReloader {
    pub fn reload_script(self, index: i32) {
        let i = index as usize
        if i < self.scripts.len() {
            self.scripts[i].reload()
        }
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn reload_script(&mut self"),
        "reload_script should infer &mut self from self.scripts[i].reload(). Generated:\n{}",
        rust
    );
}

#[test]
fn test_if_let_some_dirty_mark_transform() {
    // E0596 fix: if let Some(dirty) = opt { dirty.mark_transform() }
    // → emit if let Some(mut dirty) (ecs/world.rs pattern)
    let code = r#"
pub struct Dirty { pub transform: bool }
impl Dirty {
    pub fn new() -> Dirty { Dirty { transform: false } }
    pub fn mark_transform(self) {
        self.transform = true
    }
}
pub fn mark_dirty_if_present(opt: Option<Dirty>) {
    if let Some(dirty) = opt {
        dirty.mark_transform()
    }
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("Some(mut dirty)") || rust.contains("Some(ref mut dirty)"),
        "if let Some(dirty) with dirty.mark_transform() should emit mut binding. Generated:\n{}",
        rust
    );
}

#[test]
fn test_self_instances_indexed_sync_from_prefab() {
    // E0596 fix: self.instances[i].sync_from_prefab() → &mut self (prefab_system.rs)
    let code = r#"
pub struct PrefabInstance { pub prefab_id: i32, pub entity_id: i64, pub is_synced: bool }
impl PrefabInstance {
    pub fn new(pid: i32, eid: i64) -> PrefabInstance {
        PrefabInstance { prefab_id: pid, entity_id: eid, is_synced: false }
    }
    pub fn sync_from_prefab(self) {
        self.is_synced = true
    }
}
pub struct PrefabSystem { pub instances: Vec<PrefabInstance> }
impl PrefabSystem {
    pub fn sync_all_instances(self, prefab_id: i32) {
        for i in 0..self.instances.len() {
            if self.instances[i].prefab_id == prefab_id {
                self.instances[i].sync_from_prefab()
            }
        }
    }
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn sync_all_instances(&mut self"),
        "sync_all_instances should infer &mut self from self.instances[i].sync_from_prefab(). Generated:\n{}",
        rust
    );
}

#[test]
fn test_self_advance_to_recursive_call() {
    // E0596 fix: select_choice calls self.advance_to() → &mut self (narrative/dialog.rs)
    let code = r#"
pub struct DialogTree {
    pub nodes: Vec<DialogNode>,
    pub current_node_id: string,
}
pub struct DialogNode { pub id: string, pub choices: Vec<Choice> }
pub struct Choice { pub next_node_id: string }
impl DialogTree {
    pub fn advance_to(self, next_node_id: string) -> bool {
        self.current_node_id = next_node_id
        true
    }
    pub fn get_current_node(self) -> Option<DialogNode> { None }
    pub fn select_choice(self, choice_index: i32) -> bool {
        if let Some(node) = self.get_current_node() {
            if choice_index >= 0 && (choice_index as usize) < node.choices.len() {
                let choice = node.choices[choice_index as usize]
                return self.advance_to(choice.next_node_id.clone())
            }
        }
        false
    }
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn select_choice(&mut self"),
        "select_choice should infer &mut self from recursive self.advance_to() call. Generated:\n{}",
        rust
    );
}

#[test]
fn test_builder_with_material_set() {
    // E0596 fix: self.materials.set(id, ...) → mut self (scene/builder.rs)
    // Builder pattern: owned self + field mutation requires mut self in Rust
    // MaterialPalette has .set() method that mutates
    let code = r#"
pub struct Color { pub r: f32, pub g: f32, pub b: f32 }
pub struct VoxelMaterial { pub color: Color }
impl VoxelMaterial {
    pub fn from_color(c: Color) -> VoxelMaterial { VoxelMaterial { color: c } }
}
pub struct MaterialPalette { data: std::collections::HashMap<u8, VoxelMaterial> }
impl MaterialPalette {
    pub fn new() -> MaterialPalette { MaterialPalette { data: std::collections::HashMap::new() } }
    pub fn set(self, id: u8, m: VoxelMaterial) {
        self.data.insert(id, m)
    }
}
pub struct SceneBuilder { pub materials: MaterialPalette }
impl SceneBuilder {
    pub fn new() -> SceneBuilder {
        SceneBuilder { materials: MaterialPalette::new() }
    }
    pub fn with_material(self, id: u8, color: Color) -> SceneBuilder {
        self.materials.set(id, VoxelMaterial::from_color(color))
        SceneBuilder { materials: self.materials }
    }
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn with_material(mut self"),
        "with_material should infer mut self from self.materials.set() (builder pattern). Generated:\n{}",
        rust
    );
}

#[test]
fn test_self_components_indexed_initialize() {
    // E0596 fix: self.components[i].initialize() → &mut self (scripting/components.rs)
    let code = r#"
pub struct ScriptComponent { pub initialized: bool }
impl ScriptComponent {
    pub fn new() -> ScriptComponent { ScriptComponent { initialized: false } }
    pub fn initialize(self) {
        self.initialized = true
    }
}
pub struct ScriptSystem { pub components: Vec<ScriptComponent> }
impl ScriptSystem {
    pub fn initialize_all(self) {
        for i in 0..self.components.len() {
            if !self.components[i].initialized {
                self.components[i].initialize()
            }
        }
    }
}
"#;
    let rust = compile_windjammer_code(code).expect("Should compile");
    assert!(
        rust.contains("pub fn initialize_all(&mut self"),
        "initialize_all should infer &mut self from self.components[i].initialize(). Generated:\n{}",
        rust
    );
}
