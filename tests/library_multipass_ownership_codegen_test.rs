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

//! Ownership regressions from game-core dogfooding (E0308/E0507).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_explicit_mut_param_is_owned_binding_not_mut_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "editor/animation_timeline.wj",
        r#"
pub struct AnimationClip {
    duration: f32,
    tracks: Vec<Track>,
}

pub struct Track {
    keyframes: Vec<Keyframe>,
}

pub struct Keyframe {
    time: f32,
}

impl AnimationClip {
    pub fn new() -> AnimationClip {
        AnimationClip { duration: 0.0, tracks: Vec::new() }
    }
}

pub struct AnimationTimeline {}

impl AnimationTimeline {
    pub fn recompute_duration(self, mut clip: AnimationClip) -> AnimationClip {
        clip.duration = 1.0
        clip
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("editor/animation_timeline.rs")
        .expect("animation_timeline.rs");

    assert!(
        rs.contains("fn recompute_duration(") && rs.contains("mut clip: AnimationClip"),
        "explicit `mut clip: T` must lower to owned `mut clip: T`, not `&mut T`. Got:\n{rs}"
    );
    assert!(
        !rs.contains("mut clip: &mut AnimationClip"),
        "must not use &mut for explicit mut owned param. Got:\n{rs}"
    );
}

#[test]
fn test_builder_mutate_then_move_fields_uses_owned_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "scene/builder.wj",
        r#"
pub struct VoxelGrid {
    size: i32,
}

impl VoxelGrid {
    pub fn new() -> VoxelGrid {
        VoxelGrid { size: 1 }
    }
}

pub struct MaterialPalette {
    count: i32,
}

impl MaterialPalette {
    pub fn new() -> MaterialPalette {
        MaterialPalette { count: 0 }
    }
}

pub struct Camera {
    fov: f32,
}

impl Camera {
    pub fn new() -> Camera {
        Camera { fov: 70.0 }
    }
}

pub struct SceneBuilder {
    voxel_grids: Vec<VoxelGrid>,
    cameras: Vec<Camera>,
    materials: MaterialPalette,
}

impl SceneBuilder {
    pub fn new() -> SceneBuilder {
        SceneBuilder {
            voxel_grids: Vec::new(),
            cameras: Vec::new(),
            materials: MaterialPalette::new(),
        }
    }

    pub fn add_voxel_grid(self, grid: VoxelGrid) -> SceneBuilder {
        self.voxel_grids.push(grid)
        SceneBuilder {
            voxel_grids: self.voxel_grids,
            cameras: self.cameras,
            materials: self.materials,
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("scene/builder.rs").expect("builder.rs");

    assert!(
        rs.contains("fn add_voxel_grid(mut self") || rs.contains("fn add_voxel_grid(self"),
        "mutating then moving self fields requires owned self. Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn add_voxel_grid(&mut self"),
        "must not use &mut self when returning struct that moves self fields. Got:\n{rs}"
    );
}

#[test]
fn test_register_owned_string_local_borrows_for_str_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ecs/component_storage.wj",
        r#"
pub struct ComponentId {
    id: i32,
}

pub struct ComponentRegistry {}

impl ComponentRegistry {
    pub fn new() -> ComponentRegistry {
        ComponentRegistry {}
    }

    pub fn register(self, name: string, size: usize, align: usize) -> ComponentId {
        ComponentId { id: 1 }
    }
}
"#,
    );
    test.add_file(
        "ecs/world.wj",
        r#"
use crate::ecs::component_storage::{ComponentRegistry, ComponentId}

pub struct DynamicWorld {
    registry: ComponentRegistry,
    transform_id: ComponentId,
}

impl DynamicWorld {
    pub fn new() -> DynamicWorld {
        let mut registry = ComponentRegistry::new()
        let transform_name: string = "Transform"
        let transform_id = registry.register(transform_name, 36, 4)
        DynamicWorld { registry, transform_id }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ecs/world.rs").expect("world.rs");

    assert!(
        rs.contains("register(&transform_name") || rs.contains("register(& transform_name"),
        "owned String local must borrow as &str for register(name: &str). Got:\n{rs}"
    );
    assert!(
        !rs.contains("register(transform_name,"),
        "must not pass owned String without borrow. Got:\n{rs}"
    );
}

#[test]
fn test_component_registry_add_formal_borrows_vec_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ecs/component_storage.wj",
        r#"
pub struct ComponentId {
    id: i32,
}

pub struct ComponentArray {
    bytes: Vec<u8>,
}

impl ComponentArray {
    pub fn new() -> ComponentArray {
        ComponentArray { bytes: Vec::new() }
    }

    pub fn insert(self, entity: i64, data: Vec<u8>) {
        let mut i = 0
        while i < data.len() {
            self.bytes.push(data[i])
            i = i + 1
        }
    }
}

pub struct ComponentRegistry {
    arrays: Vec<ComponentArray>,
}

impl ComponentRegistry {
    pub fn new() -> ComponentRegistry {
        ComponentRegistry { arrays: Vec::new() }
    }

    pub fn add(self, entity: i64, component_id: ComponentId, data: Vec<u8>) {
        let idx = component_id.id as usize
        self.arrays[idx].insert(entity, data)
    }
}
"#,
    );
    test.add_file(
        "ecs/world.wj",
        r#"
use crate::ecs::component_storage::ComponentRegistry

pub fn make_registry() -> ComponentRegistry {
    ComponentRegistry::new()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ecs/component_storage.rs").expect("component_storage.rs");

    assert!(
        rs.contains("data: &Vec<u8>"),
        "add formal must emit borrowed Vec param. Got:\n{rs}"
    );
}

#[test]
fn test_registry_add_borrows_vec_data_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ecs/component_storage.wj",
        r#"
pub struct ComponentId { id: i32 }
pub struct ComponentArray { bytes: Vec<u8> }
impl ComponentArray {
    pub fn new() -> ComponentArray { ComponentArray { bytes: Vec::new() } }
    pub fn insert(self, entity: i64, data: Vec<u8>) {
        let mut i = 0
        while i < data.len() { self.bytes.push(data[i]); i = i + 1 }
    }
}
pub struct ComponentRegistry { arrays: Vec<ComponentArray> }
impl ComponentRegistry {
    pub fn new() -> ComponentRegistry { ComponentRegistry { arrays: Vec::new() } }
    pub fn add(self, entity: i64, component_id: ComponentId, data: Vec<u8>) {
        let idx = component_id.id as usize
        self.arrays[idx].insert(entity, data)
    }
    pub fn attach(self, entity: i64, transform_id: ComponentId) {
        let data: Vec<u8> = vec![0u8; 36]
        self.add(entity, transform_id, data)
    }
}
"#,
    );
    let map = test.compile().expect("compile");
    let rs = map.get("ecs/component_storage.rs").expect("component_storage.rs");
    assert!(
        rs.contains("add(entity, transform_id, &data)")
            || rs.contains("self.add(entity, transform_id, &data)"),
        "call site must borrow Vec for borrowed formal. Got:\n{rs}"
    );
}

#[test]
fn test_hashmap_get_copy_u32_key_borrows_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/navmesh.wj",
        r#"
use std::map::Map

pub fn trace_parent(came_from: Map<u32, u32>, node: u32) -> Option<u32> {
    match came_from.get(node) {
        Some(parent) => Some(parent),
        None => None,
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ai/navmesh.rs").expect("navmesh.rs");

    assert!(
        rs.contains("came_from.get(&node)"),
        "HashMap get with Copy u32 key must auto-borrow. Got:\n{rs}"
    );
}

#[test]
fn test_hashmap_get_copy_binding_push_no_spurious_deref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/path.wj",
        r#"
use std::map::Map

pub fn rebuild_path(came_from: Map<u32, u32>, goal: u32) -> Vec<u32> {
    let mut path: Vec<u32> = Vec::new()
    path.push(goal)
    let mut node = goal
    loop {
        match came_from.get(node) {
            Some(parent) => {
                path.push(parent)
                node = parent
            }
            None => break,
        }
    }
    path
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ai/path.rs").expect("path.rs");

    assert!(
        !rs.contains("*(parent).clone()"),
        "Copy HashMap binding must not get *(parent).clone(). Got:\n{rs}"
    );
    assert!(
        rs.contains("path.push(*parent)")
            || rs.contains("path.push(parent)")
            || rs.contains("path.push(parent.clone())"),
        "path.push must pass owned u32, not ref. Got:\n{rs}"
    );
}

#[test]
fn test_vec_index_return_clones_non_copy_element() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "physics/rigidbody2d.wj",
        r#"
pub struct RigidBody2D {
    mass: f32,
    label: string,
}

impl RigidBody2D {
    pub fn new() -> RigidBody2D {
        RigidBody2D { mass: 1.0 }
    }
}
"#,
    );
    test.add_file(
        "physics/physics_world.wj",
        r#"
use crate::physics::rigidbody2d::RigidBody2D

pub struct PhysicsWorld {
    bodies: Vec<RigidBody2D>,
}

impl PhysicsWorld {
    pub fn new() -> PhysicsWorld {
        PhysicsWorld { bodies: Vec::new() }
    }

    pub fn get_body(self, index: i64) -> RigidBody2D {
        return self.bodies[index as usize]
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("physics/physics_world.rs").expect("physics_world.rs");

    assert!(
        rs.contains(".clone()"),
        "return self.bodies[i] with owned non-Copy element must clone. Got:\n{rs}"
    );
    assert!(
        !rs.contains("-> RigidBody2D {\n        &self.bodies"),
        "must not return bare reference for owned return type. Got:\n{rs}"
    );
}

#[test]
fn test_copied_option_match_binding_no_spurious_deref() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "dialogue/dialogue_system.wj",
        r#"
use std::map::Map

pub struct DialogueSystem {
    node_visit_count: Map<string, u32>,
    max_visits: u32,
    active: bool,
    current_node_id: string,
}

impl DialogueSystem {
    pub fn new() -> DialogueSystem {
        DialogueSystem {
            node_visit_count: Map::new(),
            max_visits: 3,
            active: true,
            current_node_id: "start",
        }
    }

    pub fn check_visit_limit(self) {
        match self.node_visit_count.get(self.current_node_id) {
            Some(count) => {
                if count > self.max_visits {
                    self.active = false
                }
            }
            None => {}
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("dialogue/dialogue_system.rs").expect("dialogue_system.rs");

    assert!(
        !rs.contains("*count >"),
        "copied Option match binding must not deref owned Copy count. Got:\n{rs}"
    );
    assert!(
        rs.contains("count > self.max_visits") || rs.contains("count > self.max_visits"),
        "expected direct count comparison. Got:\n{rs}"
    );
}

#[test]
fn test_match_ref_struct_field_cast_derefs_before_as() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "physics/collider2d.wj",
        r#"
pub enum Collider2D {
    Box { width: f32, height: f32 },
    Circle { radius: f32 },
}
"#,
    );
    test.add_file(
        "physics/character2d.wj",
        r#"
use crate::physics::collider2d::Collider2D

pub struct Character2D {
    collider: Collider2D,
}

impl Character2D {
    pub fn dims(self) -> f32 {
        match &self.collider {
            Collider2D::Box { width: w, height: h } => w / 2.0 + h as f32 / 2.0,
            Collider2D::Circle { radius: r } => r,
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("physics/character2d.rs").expect("character2d.rs");

    assert!(
        rs.contains("*h as f32"),
        "expected deref on ref match binding used in cast. Got:\n{rs}"
    );
    assert!(
        !rs.contains("+ h as f32"),
        "must not cast ref binding without deref. Got:\n{rs}"
    );
}

#[test]
fn test_match_enum_field_on_borrowed_self_collider_cast_derefs() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "physics/rigidbody2d.wj",
        r#"
pub enum Collider2D {
    Box { width: f32, height: f32 },
    Circle { radius: f32 },
}

pub struct RigidBody2D {
    collider: Collider2D,
    is_kinematic: bool,
}

impl RigidBody2D {
    pub fn new_box(position: f32, mass: f32, width: f32, height: f32) -> RigidBody2D {
        RigidBody2D {
            collider: Collider2D::Box { width: width, height: height },
        }
    }
}
"#,
    );
    test.add_file(
        "physics/physics_world.wj",
        r#"
pub struct PhysicsWorld {}
"#,
    );
    test.add_file(
        "physics/character2d.wj",
        r#"
use crate::physics::rigidbody2d::RigidBody2D
use crate::physics::rigidbody2d::Collider2D
use crate::physics::physics_world::PhysicsWorld

pub struct CharacterController2D {
    body: RigidBody2D,
    is_grounded: bool,
}

impl CharacterController2D {
    pub fn check_ground(self, world: PhysicsWorld) {
        self.is_grounded = false
        let (half_width, half_height) = match self.body.collider {
            Collider2D::Box { width: w, height: h } => (w / 2.0, h / 2.0),
            Collider2D::Circle { radius: r } => (r / 1.0, r / 1.0),
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("physics/character2d.rs").expect("character2d.rs");

    assert!(
        !rs.contains("h as f32") || rs.contains("*h as f32"),
        "match binding behind &scrutinee must deref before cast. Got:\n{rs}"
    );
    assert!(
        !rs.contains("w as f32") || rs.contains("*w as f32"),
        "match binding behind &scrutinee must deref before cast. Got:\n{rs}"
    );
}

#[test]
fn test_inner_loop_i32_counter_shadowing_f32_name_no_float_cast() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "world/streaming.wj",
        r#"
pub struct Vec3 {
    x: f32,
    z: f32,
}

pub struct StreamingTileBatch {
    add_ids: Vec<u32>,
}

impl StreamingTileBatch {
    pub fn new() -> StreamingTileBatch {
        StreamingTileBatch { add_ids: Vec::new() }
    }
}

pub struct StreamingCoordinator {}

impl StreamingCoordinator {
    pub fn update(self, position: Vec3, load_radius: f32) -> StreamingTileBatch {
        let cell_size = 128.0
        let cx = (position.x / cell_size).floor() as i32
        let cz = (position.z / cell_size).floor() as i32
        let dx = position.x
        let dz = position.z
        let mut batch = StreamingTileBatch::new()
        if load_radius > 0.0 {
            let r = (load_radius / cell_size).ceil() as i32
            let mut dz = -r
            while dz <= r {
                let mut dx = -r
                while dx <= r {
                    let id = ((cx + dx) as u32) * 10000 + ((cz + dz) as u32)
                    batch.add_ids.push(id)
                    dx = dx + 1
                }
                dz = dz + 1
            }
        }
        batch
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("world/streaming.rs").expect("streaming.rs");

    assert!(
        !rs.contains("dx += 1 as f32") && !rs.contains("(cx as f32)"),
        "inner i32 loop counters must not get float casts. Got:\n{rs}"
    );
}

#[test]
fn test_method_calling_mutating_helper_uses_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/game_renderer.wj",
        r#"
pub struct MeshBatch {}

impl MeshBatch {
    pub fn new() -> MeshBatch {
        MeshBatch {}
    }

    pub fn clear(self) {
    }
}

pub struct GameRenderer {
    mesh_batch: MeshBatch,
    camera_updated_this_frame: bool,
}

impl GameRenderer {
    pub fn new() -> GameRenderer {
        GameRenderer {
            mesh_batch: MeshBatch::new(),
            camera_updated_this_frame: false,
        }
    }

    pub fn begin_frame(self) {
        self.mesh_batch.clear()
        self.camera_updated_this_frame = false
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("rendering/game_renderer.rs").expect("game_renderer.rs");

    assert!(
        rs.contains("fn begin_frame(&mut self"),
        "mutating self fields and calling mutating methods requires &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_csg_evaluate_owned_self_when_calling_owned_evaluate_node() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "csg/evaluator.wj",
        r#"
pub struct CsgScene {
    root_id: i32,
}

impl CsgScene {
    pub fn root(self) -> i32 {
        self.root_id
    }
}

pub struct CsgEvaluator {
    scene: CsgScene,
}

impl CsgEvaluator {
    pub fn new(scene: CsgScene) -> CsgEvaluator {
        CsgEvaluator { scene: scene }
    }

    pub fn evaluate(self, x: f32, y: f32, z: f32) -> (f32, u8) {
        self.evaluate_node(self.scene.root(), x, y, z)
    }

    fn evaluate_node(self, node_id: i32, x: f32, y: f32, z: f32) -> (f32, u8) {
        if node_id <= 0 {
            return (x + y + z, 0)
        }
        self.evaluate_node(node_id - 1, x, y, z)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("csg/evaluator.rs").expect("evaluator.rs");

    assert!(
        rs.contains("fn evaluate_node(&self,")
            || rs.contains("fn evaluate_node(self,"),
        "read-only recursive evaluate_node should get &self or self (Copy). Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn evaluate_node(&mut self,"),
        "read-only evaluate_node should not get &mut self. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn evaluate(&self,")
            || rs.contains("fn evaluate(self,"),
        "evaluate (calling read-only evaluate_node) should get &self or self (Copy). Got:\n{rs}"
    );
}

#[test]
fn test_csg_evaluate_node_multiple_recursive_calls_use_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "csg/evaluator.wj",
        r#"
pub struct CsgScene {
    root_id: i32,
}

impl CsgScene {
    pub fn get_node(self, id: i32) -> Option<i32> {
        if id == self.root_id {
            Some(id)
        } else {
            None
        }
    }

    pub fn root(self) -> i32 {
        self.root_id
    }
}

pub struct CsgEvaluator {
    scene: CsgScene,
}

impl CsgEvaluator {
    pub fn evaluate(self, x: f32, y: f32, z: f32) -> (f32, u8) {
        self.evaluate_node(self.scene.root(), x, y, z)
    }

    fn evaluate_node(self, node_id: i32, x: f32, y: f32, z: f32) -> (f32, u8) {
        if node_id <= 0 {
            return (x + y + z, 0)
        }
        let (d1, m1) = self.evaluate_node(node_id - 1, x, y, z)
        let (d2, m2) = self.evaluate_node(node_id - 2, x, y, z)
        (d1 + d2, m1 + m2)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("csg/evaluator.rs").expect("evaluator.rs");

    assert!(
        rs.contains("fn evaluate_node(&self,")
            || rs.contains("fn evaluate_node(self,"),
        "read-only recursive evaluate_node (even with two calls) should get &self or self (Copy). Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn evaluate_node(&mut self,"),
        "read-only evaluate_node should not get &mut self. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn evaluate(&self,")
            || rs.contains("fn evaluate(self,"),
        "evaluate (calling read-only evaluate_node) should get &self or self (Copy). Got:\n{rs}"
    );
}

#[test]
fn test_match_borrow_break_ref_binding_clones_non_copy_in_tuple() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "assets/loader.wj",
        r#"
pub enum AssetError {
    NotFound,
}

pub struct LoadedAsset {
    name: string,
}

pub struct BatchLoadResult {
    failed: Vec<(string, AssetError)>,
}

impl BatchLoadResult {
    pub fn new() -> BatchLoadResult {
        BatchLoadResult { failed: Vec::new() }
    }
}

pub struct AssetLoader {
    error_count: i32,
}

impl AssetLoader {
    pub fn new() -> AssetLoader {
        AssetLoader { error_count: 0 }
    }

    pub fn load(self, name: string, path: string, size: i32) -> Result<LoadedAsset, AssetError> {
        self.error_count = self.error_count + 1
        Ok(LoadedAsset { name: name })
    }

    pub fn load_batch(self, assets: Vec<(string, string, i32)>) -> BatchLoadResult {
        let mut failures: Vec<(string, AssetError)> = Vec::new()
        for entry in assets {
            let (name, path, size) = entry
            match self.load(name, path, size) {
                Ok(_asset) => {},
                Err(error) => {
                    self.error_count = self.error_count + 1
                    failures.push((name, error))
                },
            }
        }
        BatchLoadResult { failed: failures }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("assets/loader.rs").expect("loader.rs");

    assert!(
        rs.contains("__match_borrow_break") && rs.contains("error.clone()"),
        "Err binding behind borrow-break .as_ref() must clone for owned tuple field. Got:\n{rs}"
    );
}

#[test]
fn test_match_borrow_break_copy_index_derefs_for_array_store() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "tilemap/tilemap.wj",
        r#"
pub struct Tile {
    solid: bool,
}

impl Tile {
    pub fn new(solid: bool) -> Tile {
        Tile { solid: solid }
    }
}

pub struct Tilemap {
    tiles: Vec<Option<Tile>>,
    width: i32,
}

impl Tilemap {
    pub fn new(width: i32) -> Tilemap {
        Tilemap { tiles: Vec::new(), width: width }
    }

    fn tile_to_index(self, x: i32, y: i32) -> Option<i32> {
        Some(x + y * self.width)
    }

    pub fn set_tile(self, x: i32, y: i32, tile: Option<Tile>) {
        match self.tile_to_index(x, y) {
            Some(index) => {
                self.tiles[index] = tile
            },
            None => {},
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("tilemap/tilemap.rs").expect("tilemap.rs");

    assert!(
        !rs.contains("tiles[index]") || rs.contains("*index") || rs.contains("tiles[(*index"),
        "Copy index binding behind & must deref for Vec indexing. Got:\n{rs}"
    );
}

#[test]
fn test_match_borrow_break_self_index_clone_yields_owned_enum_bindings() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "animation/blend_tree.wj",
        r#"
pub enum BlendNode {
    Clip { animation_id: u32 },
    Lerp { node_a: u32, node_b: u32, blend_factor: f32 },
}

pub struct AnimationClip {
    name: string,
}

pub struct BoneTransform {
    x: f32,
}

pub struct BlendTree {
    nodes: Vec<BlendNode>,
}

impl BlendTree {
    pub fn new() -> BlendTree {
        BlendTree { nodes: Vec::new() }
    }

    fn evaluate_node(self, node_id: u32, clips: Vec<AnimationClip>, time: f32, bone_count: u32) -> Vec<BoneTransform> {
        if node_id as usize >= self.nodes.len() {
            return Vec::new()
        }
        match self.nodes[node_id as usize] {
            BlendNode::Clip { animation_id } => {
                self.evaluate_node(animation_id, clips, time, bone_count)
            },
            BlendNode::Lerp { node_a, node_b, blend_factor } => {
                let _ = self.evaluate_node(node_a, clips, time, bone_count)
                let _ = self.evaluate_node(node_b, clips, time, bone_count)
                Vec::new()
            },
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("animation/blend_tree.rs").expect("blend_tree.rs");

    assert!(
        rs.contains("let mut __match_borrow_break = self.nodes[") && rs.contains("].clone();"),
        "enum borrow-break must clone owned node into mut binding for ref mut patterns. Got:\n{rs}"
    );
    assert!(
        !rs.contains("let __match_borrow_break = &self.nodes"),
        "must not match on reference to cloned enum (ref pattern bindings). Got:\n{rs}"
    );
}

#[test]
fn test_owned_match_binding_struct_literal_no_deref_on_copy_fields() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "animation/blend_tree.wj",
        r#"
pub enum BlendNode {
    Lerp { node_a: u32, node_b: u32, blend_factor: f32 },
}

pub struct BlendTree {
    nodes: Vec<BlendNode>,
}

impl BlendTree {
    pub fn new() -> BlendTree {
        BlendTree { nodes: Vec::new() }
    }

    pub fn set_blend_factor(self, node_id: u32, value: f32) {
        if node_id as usize >= self.nodes.len() {
            return
        }
        match self.nodes[node_id as usize] {
            BlendNode::Lerp { node_a, node_b, blend_factor } => {
                self.nodes[node_id as usize] = BlendNode::Lerp {
                    node_a: node_a,
                    node_b: node_b,
                    blend_factor: value,
                }
            },
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("animation/blend_tree.rs").expect("blend_tree.rs");

    assert!(
        !rs.contains("*node_a") && !rs.contains("*(node_a)"),
        "owned match bindings must not deref Copy fields in struct literal. Got:\n{rs}"
    );
    assert!(
        rs.contains("node_a: node_a") || rs.contains("node_a, node_b"),
        "expected plain owned binding reuse. Got:\n{rs}"
    );
}

#[test]
fn test_if_let_self_world_apply_consequences_uses_mut_scrutinee() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "dialogue/dialogue.wj",
        r#"
pub struct World {
    data: Vec<i32>,
}

impl World {
    pub fn new() -> World {
        World { data: Vec::new() }
    }

    pub fn set_variable(self, key: string, value: string) {
        self.data.push(1)
    }
}

pub struct Choice {}

impl Choice {
    pub fn apply_consequences(self, world: World) {
        world.set_variable("k", "v")
    }
}

pub struct DialogueSystem {
    world: Option<World>,
}

impl DialogueSystem {
    pub fn make_choice(self, choice: Choice) {
        if let Some(world) = self.world {
            choice.apply_consequences(world)
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("dialogue/dialogue.rs").expect("dialogue.rs");

    assert!(
        rs.contains("&mut self.world"),
        "if let Some(world) must use &mut scrutinee when passed to apply_consequences. Got:\n{rs}"
    );
}

#[test]
fn test_borrowed_fn_call_gets_ref_for_owned_local() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "physics/collision.wj",
        r#"
pub enum Collider2D {
    Box { w: f32 },
}

pub struct RigidBody2D {
    collider: Collider2D,
    is_kinematic: bool,
}

pub fn check_collision(a: RigidBody2D, b: RigidBody2D) -> bool {
    match a.collider {
        Collider2D::Box { w } => {
            match b.collider {
                Collider2D::Box { w: w2 } => w == w2,
            }
        },
    }
}

pub fn run() -> bool {
    let a = RigidBody2D { collider: Collider2D::Box { w: 1.0 }, is_kinematic: false }
    let b = RigidBody2D { collider: Collider2D::Box { w: 1.0 }, is_kinematic: false }
    check_collision(a, b)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("physics/collision.rs").expect("collision.rs");

    // RigidBody2D is Copy — owned (by value) is correct and more efficient
    assert!(
        rs.contains("fn check_collision(a: &RigidBody2D, b: &RigidBody2D)")
            || rs.contains("fn check_collision(a: RigidBody2D, b: RigidBody2D)"),
        "match-on-field should infer borrowed or owned (Copy) params. Got:\n{rs}"
    );
    if rs.contains("a: &RigidBody2D") {
        assert!(
            rs.contains("check_collision(&a, &b)")
                || rs.contains("check_collision(& a, & b)"),
            "borrowed callee params must get & at call site. Got:\n{rs}"
        );
    }
}

#[test]
fn test_cross_module_imported_borrowed_fn_gets_ref_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "physics/collision2d.wj",
        r#"
pub enum Collider2D {
    Box { w: f32 },
}

pub struct RigidBody2D {
    collider: Collider2D,
    is_kinematic: bool,
}

pub fn check_collision(a: RigidBody2D, b: RigidBody2D) -> Option<bool> {
    match a.collider {
        Collider2D::Box { w } => {
            match b.collider {
                Collider2D::Box { w: w2 } => Some(w == w2),
            }
        },
    }
}
"#,
    );
    test.add_file(
        "physics/physics_world.wj",
        r#"
use collision2d::check_collision
use collision2d::RigidBody2D
use collision2d::Collider2D

pub struct PhysicsWorld {
    bodies: Vec<RigidBody2D>,
}

impl PhysicsWorld {
    pub fn new() -> PhysicsWorld {
        PhysicsWorld { bodies: Vec::new() }
    }

    pub fn detect(self) -> bool {
        for i in 0..self.bodies.len() {
            let body_a = self.bodies[i]
            for j in (i + 1)..self.bodies.len() {
                let body_b = self.bodies[j]
                if body_a.is_kinematic && body_b.is_kinematic {
                    continue
                }
                match check_collision(body_a, body_b) {
                    Some(_) => {},
                    None => {},
                }
            }
        }
        true
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("physics/physics_world.rs").expect("physics_world.rs");

    assert!(
        rs.contains("check_collision(&body_a, &body_b)")
            || rs.contains("check_collision(& body_a, & body_b)"),
        "imported borrowed callee must get & for owned locals. Got:\n{rs}"
    );
    assert!(
        !rs.contains("check_collision(body_a.clone()"),
        "must not pass owned clone to borrowed params. Got:\n{rs}"
    );
}

#[test]
fn test_imported_borrowed_fn_borrows_cloned_locals_on_borrowed_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "physics/collision2d.wj",
        r#"
pub enum Collider2D {
    Box { w: f32 },
}

pub struct RigidBody2D {
    collider: Collider2D,
    is_kinematic: bool,
}

pub fn check_collision(a: RigidBody2D, b: RigidBody2D) -> Option<bool> {
    match a.collider {
        Collider2D::Box { w } => {
            match b.collider {
                Collider2D::Box { w: w2 } => Some(w == w2),
            }
        },
    }
}
"#,
    );
    test.add_file(
        "physics/physics_body.wj",
        r#"
pub struct VoxelWorld {}

pub struct PhysicsBody {}

impl PhysicsBody {
    fn check_collision(self, world: VoxelWorld) {
    }
}
"#,
    );
    test.add_file(
        "physics/physics_world.wj",
        r#"
use collision2d::check_collision
use collision2d::RigidBody2D
use collision2d::Collider2D

pub struct PhysicsWorld {
    bodies: Vec<RigidBody2D>,
}

impl PhysicsWorld {
    pub fn detect_collisions(self) -> bool {
        for i in 0..self.bodies.len() {
            let body_a = self.bodies[i]
            for j in (i + 1)..self.bodies.len() {
                let body_b = self.bodies[j]
                if body_a.is_kinematic && body_b.is_kinematic {
                    continue
                }
                match check_collision(body_a, body_b) {
                    Some(_) => {},
                    None => {},
                }
            }
        }
        true
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("physics/physics_world.rs").expect("physics_world.rs");

    assert!(
        rs.contains("check_collision(&body_a, &body_b)")
            || rs.contains("check_collision(& body_a, & body_b)"),
        "auto-cloned locals must borrow into imported &T params. Got:\n{rs}"
    );
    assert!(
        !rs.contains("check_collision(body_a.clone()"),
        "must not pass owned clone to borrowed params. Got:\n{rs}"
    );
}

#[test]
fn test_mut_string_local_from_borrowed_param_coerces_to_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "dialogue/dialogue.wj",
        r#"
pub struct World {}

impl World {
    pub fn new() -> World {
        World {}
    }
}

pub struct DialogueSystem {}

impl DialogueSystem {
    pub fn new() -> DialogueSystem {
        DialogueSystem {}
    }

    fn substitute_variables(self, text: string, _world: World) -> string {
        let mut result = text
        result
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("dialogue/dialogue.rs").expect("dialogue.rs");

    assert!(
        rs.contains("let mut result = text.to_string()")
            || rs.contains("return result.to_string()")
            || rs.contains("return text.to_string()"),
        "mutable string local + String return must coerce &str → String. Got:\n{rs}"
    );
}

#[test]
fn test_profile_decorator_impl_method_uses_mut_borrow_not_typed_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/gpu_passes.wj",
        r#"
pub struct GpuPasses {
    count: i32,
}

impl GpuPasses {
    @profile("update_params")
    pub fn update_all_params(self) {
        self.count = 1
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("rendering/gpu_passes.rs").expect("gpu_passes.rs");

    assert!(
        rs.contains("pub fn update_all_params(&mut self)")
            || rs.contains("pub fn update_all_params( &mut self )"),
        "@profile must reuse self-aware receiver lowering. Got:\n{rs}"
    );
    assert!(
        !rs.contains("mut self: Self"),
        "must not emit typed owned self receiver. Got:\n{rs}"
    );
}

#[test]
fn test_same_call_move_and_field_read_clones_root_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/bt_validation.wj",
        r#"
pub struct BtDocument {
    root_id: i32,
    nodes: Vec<i32>,
}

pub fn visit_cycle(doc: BtDocument, cur: i32) -> bool {
    cur == doc.root_id
}

pub fn consume(_doc: BtDocument) {}

pub fn bt_document_contains_cycle(doc: BtDocument) -> bool {
    let result = visit_cycle(doc, doc.root_id)
    consume(doc)
    result
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ai/bt_validation.rs").expect("bt_validation.rs");

    assert!(
        rs.contains("visit_cycle(doc, doc.root_id")
            || rs.contains("visit_cycle(doc.clone(), doc.root_id")
            || rs.contains("visit_cycle(&doc, doc.root_id"),
        "same-call root + field must compile (borrow param, &, or clone). Got:\n{rs}"
    );
    assert!(
        rs.contains("doc: &BtDocument")
            || rs.contains("visit_cycle(doc.clone(),")
            || rs.contains("visit_cycle(&doc,"),
        "owned root reused in one call must not move before field read. Got:\n{rs}"
    );
}

#[test]
fn test_for_in_over_indexed_field_borrows_iterable() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/shader_graph_builder.wj",
        r#"
pub enum PassId {
    A,
}

pub struct PassDefinition {
    pass_id: PassId,
    dependencies: Vec<PassId>,
}

pub fn count_deps(pass_defs: Vec<PassDefinition>) -> i32 {
    let mut total = 0
    let mut pi = 0
    while pi < pass_defs.len() {
        for dep_id in pass_defs[pi].dependencies {
            total = total + 1
        }
        pi = pi + 1
    }
    total
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/shader_graph_builder.rs")
        .expect("shader_graph_builder.rs");

    assert!(
        rs.contains("for dep_id in &pass_defs[pi].dependencies")
            || rs.contains("for dep_id in & pass_defs[pi].dependencies")
            || rs.contains("for _dep_id in &pass_defs[pi].dependencies")
            || rs.contains("for _dep_id in & pass_defs[pi].dependencies"),
        "indexed field iteration must borrow to avoid partial move. Got:\n{rs}"
    );
}

#[test]
fn test_copy_field_preferred_over_owned_method_on_ref_receiver() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/gpu_types.wj",
        r#"
pub struct StorageRead<T> {
    buffer_id: u32,
    data: T,
}

impl StorageRead<T> {
    pub fn buffer_id(self) -> u32 {
        self.buffer_id
    }
}

pub struct PassBuilder {}

impl PassBuilder {
    pub fn bind(self, buffer: StorageRead<i32>) -> PassBuilder {
        let _id = buffer.buffer_id()
        self
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("rendering/gpu_types.rs").expect("gpu_types.rs");

    assert!(
        rs.contains("buffer.buffer_id") && !rs.contains("buffer.buffer_id()"),
        "Copy field must win over owned method on & receiver. Got:\n{rs}"
    );
}

#[test]
fn test_mutating_impl_method_call_in_let_needs_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "editor/scene_editor.wj",
        r#"
pub struct Scene {
    next_id: i64,
}

impl Scene {
    pub fn new() -> Scene {
        Scene { next_id: 0 }
    }
    pub fn create_entity(&mut self) -> i64 {
        let id = self.next_id
        self.next_id = id + 1
        id
    }
}

pub struct SceneEditorState {
    scene: Scene,
    selected_entities: Vec<i64>,
}

impl SceneEditorState {
    pub fn duplicate_entity(&mut self, _entity_id: i64) -> i64 {
        self.scene.create_entity()
    }

    pub fn duplicate_selected(self) -> Vec<i64> {
        let mut out: Vec<i64> = Vec::new()
        for entity_id in self.selected_entities {
            let new_id = self.duplicate_entity(entity_id)
            out.push(new_id)
        }
        out
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("editor/scene_editor.rs").expect("scene_editor.rs");

    assert!(
        rs.contains("pub fn duplicate_selected(&mut self)"),
        "mutating method calls in body must infer &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_nested_self_field_snapshot_clones_before_owned_call() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "editor/editor_core.wj",
        r#"
pub struct Scene {
    entities: Vec<i64>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene { entities: Vec::new() }
    }
    pub fn snapshot(self) -> Scene {
        Scene { entities: self.entities }
    }
}

pub struct SceneEditorState {
    scene: Scene,
}

impl SceneEditorState {
    pub fn new() -> SceneEditorState {
        SceneEditorState { scene: Scene::new() }
    }
}

pub struct EditorState {
    scene_editor: SceneEditorState,
    play_mode_scene_snapshot: Option<Scene>,
}

impl EditorState {
    pub fn enter_play_mode(&mut self) {
        self.play_mode_scene_snapshot = Some(self.scene_editor.scene.snapshot())
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("editor/editor_core.rs").expect("editor_core.rs");

    assert!(
        rs.contains("self.scene_editor.scene.clone().snapshot()")
            || rs.contains("self.scene_editor.scene.clone(). snapshot()"),
        "nested field + owned snapshot must clone field first. Got:\n{rs}"
    );
}

#[test]
fn test_let_param_alias_then_reuse_clones_at_alias() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/squad_tactics.wj",
        r#"
pub enum MessageType {
    Ping,
}

pub struct Message {
    message_type: MessageType,
}

pub struct Squad {
    messages: Vec<Message>,
}

impl Squad {
    pub fn send_message(&mut self, message: Message) {
        let msg_copy = message
        match msg_copy.message_type {
            MessageType::Ping => {},
        }
        self.messages.push(message)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ai/squad_tactics.rs").expect("squad_tactics.rs");

    assert!(
        rs.contains("let msg_copy = message.clone()")
            || rs.contains("self.messages.push(message.clone())"),
        "param moved to alias then reused must auto-clone. Got:\n{rs}"
    );
}

#[test]
fn test_for_mut_over_self_field_on_mut_self_borrows_iterable() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ai/squad_tactics.wj",
        r#"
pub enum AlertLevel {
    Unaware,
    Suspicious,
}

pub struct SquadMember {
    alert_level: AlertLevel,
}

pub struct Squad {
    members: Vec<SquadMember>,
    shared_target_position: Option<f32>,
}

impl Squad {
    pub fn alert_members(&mut self) {
        for member in self.members {
            if member.alert_level == AlertLevel::Unaware {
                member.alert_level = AlertLevel::Suspicious
            }
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ai/squad_tactics.rs").expect("squad_tactics.rs");

    assert!(
        rs.contains("for mut member in &mut self.members")
            || rs.contains("for member in &mut self.members"),
        "mutating loop var over self field on &mut self must borrow mutably. Got:\n{rs}"
    );
    assert!(
        !rs.contains("for mut member in self.members")
            && !rs.contains("for member in self.members {"),
        "must not move self.members off &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_struct_literal_then_reuse_clones_at_first_move() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "event/dispatcher.wj",
        r#"
pub struct GameEvent {
    text: string,
}

impl GameEvent {
    pub fn describe(self) -> string {
        self.text
    }
}

pub struct EventLogEntry {
    event_description: string,
}

pub struct Dispatcher {
    event_log: Vec<EventLogEntry>,
    max_log_size: i32,
    current_time: f32,
}

impl Dispatcher {
    pub fn record(self, event: GameEvent) -> Vec<string> {
        let mut descriptions: Vec<string> = Vec::new()
        let desc = event.describe()
        if self.event_log.len() < self.max_log_size {
            self.event_log.push(EventLogEntry { event_description: desc, timestamp: self.current_time, handled: true })
        }
        descriptions.push(desc)
        descriptions
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("event/dispatcher.rs").expect("dispatcher.rs");

    assert!(
        rs.contains("event_description: desc.clone()")
            || rs.contains("EventLogEntry { event_description: desc.clone()"),
        "desc moved into struct then reused must clone at struct field. Got:\n{rs}"
    );
}

#[test]
fn test_tuple_insert_then_reuse_clones_at_insert() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "save/manager.wj",
        r#"
pub struct SaveData {
    bytes: i32,
}

pub struct SaveMetadata {
    slot_name: string,
}

impl SaveMetadata {
    pub fn new() -> SaveMetadata {
        SaveMetadata { slot_name: "" }
    }
    pub fn set_slot_name(self, name: string) {
        self.slot_name = name
    }
}

pub struct SaveManager {
    slots: Map<usize, (SaveData, SaveMetadata)>,
}

impl SaveManager {
    pub fn new() -> SaveManager {
        SaveManager { slots: Map::new() }
    }

    pub fn write_slot_to_disk(self, slot: usize, data: SaveData) {
    }

    pub fn save_to_slot(&mut self, slot: usize, data: SaveData) {
        let mut metadata = SaveMetadata::new()
        self.slots.insert(slot, (data, metadata))
        self.write_slot_to_disk(slot, data)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("save/manager.rs").expect("manager.rs");

    assert!(
        rs.contains("(data.clone(), metadata)")
            || rs.contains("(data.clone(), metadata.clone())"),
        "insert tuple must clone param reused after move. Got:\n{rs}"
    );
}

#[test]
fn test_pass_method_consuming_self_in_struct_literal_uses_owned_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "render/shader_graph_builder.wj",
        r#"
pub struct PassId {
    id: i32,
}

pub struct PassBuilder {
    graph: ShaderGraphBuilder,
    pass_id: PassId,
}

impl PassBuilder {
    pub fn new() -> PassBuilder {
        PassBuilder {
            graph: ShaderGraphBuilder { passes: Vec::new() },
            pass_id: PassId { id: 0 },
        }
    }
}

pub struct ShaderGraphBuilder {
    passes: Vec<i32>,
}

impl ShaderGraphBuilder {
    pub fn pass(self, id: PassId) -> PassBuilder {
        PassBuilder { graph: self, pass_id: id }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("render/shader_graph_builder.rs")
        .expect("shader_graph_builder.rs");

    assert!(
        rs.contains("fn pass(self, id: PassId)") || rs.contains("fn pass(mut self, id: PassId)"),
        "pass() moves self into PassBuilder and must take owned self. Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn pass(&self, id: PassId)"),
        "pass() must not use &self when graph: self moves builder. Got:\n{rs}"
    );
}

#[test]
fn test_get_compositor_clones_nested_field_on_borrowed_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "render/unified_renderer.wj",
        r#"
pub struct HybridCompositor {
    width: i32,
}

impl HybridCompositor {
    pub fn new() -> HybridCompositor {
        HybridCompositor { width: 1280 }
    }
}

pub struct VoxelGPURenderer {
    compositor: HybridCompositor,
}

impl VoxelGPURenderer {
    pub fn new() -> VoxelGPURenderer {
        VoxelGPURenderer { compositor: HybridCompositor::new() }
    }
}

pub struct UnifiedRenderer {
    voxel_renderer: VoxelGPURenderer,
}

impl UnifiedRenderer {
    pub fn get_compositor(self) -> HybridCompositor {
        self.voxel_renderer.compositor
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("render/unified_renderer.rs").expect("unified_renderer.rs");

    // For Copy types, direct field access works without .clone()
    assert!(
        rs.contains("compositor.clone()") || rs.contains("self.voxel_renderer.compositor"),
        "borrowed self returning nested field: .clone() or direct Copy access. Got:\n{rs}"
    );
}

#[test]
fn test_self_field_max_does_not_upgrade_to_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "render/sky_model.wj",
        r#"
pub struct SkyModel {
    turbidity: f32,
    sun_elevation: f32,
    intensity: f32,
}

impl SkyModel {
    pub fn zenith_color(self) -> (f32, f32, f32) {
        let t = self.turbidity
        let theta_s = self.sun_elevation.max(0.01)
        let chi = (4.0 / 9.0 - t / 120.0) * (3.14159 - 2.0 * theta_s)
        let zenith_lum = (4.0453 * t - 4.9710) * chi.tan() - 0.2155 * t + 2.4192
        let lum = zenith_lum.max(0.0) * self.intensity
        (lum, lum, lum)
    }

    pub fn to_gpu_data(self) -> Vec<f32> {
        let (_, sky_g, _) = self.zenith_color()
        let mut data = Vec::new()
        data.push(sky_g)
        data
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("render/sky_model.rs").expect("sky_model.rs");

    assert!(
        rs.contains("fn zenith_color(&self)"),
        "self.field.max() is read-only and must not infer &mut self. Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn zenith_color(&mut self)"),
        "zenith_color must not use &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_tuple_match_option_mut_ref_scrutinee_skips_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "render/skinned_mesh_renderer.wj",
        r#"
pub struct Clip {
    duration: f32,
}

impl Clip {
    pub fn sample_bone(self, bone: u32, time: f32) -> Option<i32> {
        None
    }
}

pub struct Skeleton {
    bones: Vec<i32>,
}

impl Skeleton {
    pub fn update(self) {
    }
    pub fn compute_skinning_palette(self) -> Vec<f32> {
        Vec::new()
    }
}

pub struct Instance {
    active_clip: string,
    skeleton_id: string,
}

pub struct SkinnedMeshRenderer {
    clips: Map<string, Clip>,
    skeletons: Map<string, Skeleton>,
    instances: Vec<Instance>,
}

impl SkinnedMeshRenderer {
    pub fn update_all(self) {
        let clip_opt = self.clips.get("a")
        let skel_opt = self.skeletons.get_mut("b")
        match (clip_opt, skel_opt) {
            (Some(clip), Some(skel)) => {
                skel.update()
            },
            _ => {},
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("render/skinned_mesh_renderer.rs")
        .expect("skinned_mesh_renderer.rs");

    assert!(
        !rs.contains("skel_opt.clone()"),
        "Option<&mut T> scrutinee must not be cloned in tuple match. Got:\n{rs}"
    );
}

#[test]
fn test_static_self_method_borrows_str_param_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "editor/asset_browser.wj",
        r#"
impl AssetEntry {
    pub fn new(id: i32, name: string, path: string) -> AssetEntry {
        let ext = Self::extract_extension(path)
        AssetEntry { id: id, name: name, path: path, asset_type: 0, size_bytes: 0, is_loaded: false, visible: true, dependencies: Vec::new() }
    }

    pub fn extract_extension(path: string) -> string {
        path
    }

    pub fn get_extension(self) -> string {
        Self::extract_extension(self.path)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("editor/asset_browser.rs").expect("asset_browser.rs");

    assert!(
        rs.contains("Self::extract_extension(path)") || rs.contains("Self::extract_extension(&path)"),
        "static call with borrowed string param must not clone/to_string. Got:\n{rs}"
    );
    assert!(
        !rs.contains("path.to_string().clone()") && !rs.contains("self.path.clone()"),
        "must borrow &str at static call sites. Got:\n{rs}"
    );
}

#[test]
fn test_static_self_method_borrows_vec_param_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/build_fingerprint.wj",
        r#"
impl BuildFingerprint {
    pub fn generate(source_dir: string) -> BuildFingerprint {
        let files = Self::collect_wj_files(source_dir)
        let hash = Self::hash_files(files)
        BuildFingerprint { source_hash: hash, build_timestamp: 0, source_files: files }
    }

    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }

    fn hash_files(files: Vec<string>) -> u64 {
        0
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/build_fingerprint.rs")
        .expect("build_fingerprint.rs");

    assert!(
        rs.contains("Self::collect_wj_files(source_dir)")
            || rs.contains("Self::collect_wj_files(&source_dir)"),
        "borrowed string static arg must not to_string. Got:\n{rs}"
    );
    assert!(
        rs.contains("Self::hash_files(&files)") || rs.contains("Self::hash_files(files.as_ref())"),
        "borrowed Vec param must use reference not clone. Got:\n{rs}"
    );
    assert!(
        !rs.contains("hash_files(files.clone())"),
        "must not clone Vec for borrowed param. Got:\n{rs}"
    );
}

#[test]
fn test_impl_clone_where_on_method_not_whole_impl() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/gpu_types.wj",
        r#"
pub struct StorageWrite<T> {
    buffer_id: u32,
    data: T,
}

impl StorageWrite<T> {
    pub fn from_id(buffer_id: u32, data: T) -> StorageWrite<T> {
        StorageWrite { buffer_id: buffer_id, data: data }
    }
}

pub struct StorageRead<T> {
    buffer_id: u32,
    data: T,
}

impl StorageRead<T> {
    pub fn from(write: StorageWrite<T>) -> StorageRead<T> {
        StorageRead { buffer_id: write.buffer_id, data: write.data.clone() }
    }

    pub fn buffer_id(self) -> u32 {
        self.buffer_id
    }
}

pub struct PassBuilder {}

impl PassBuilder {
    pub fn bind(self, buffer: StorageRead<i32>) -> PassBuilder {
        let _id = buffer.buffer_id()
        self
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("rendering/gpu_types.rs").expect("gpu_types.rs");

    assert!(
        !rs.contains("impl<T> StorageRead<T>\nwhere\n    T: Clone"),
        "T: Clone must not apply to whole impl. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn from(") && rs.contains("where\n    T: Clone"),
        "only `from` needs T: Clone. Got:\n{rs}"
    );
}

#[test]
fn test_self_indexed_field_mutating_method_needs_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "editor/viewport.wj",
        r#"
pub struct Vec2 {
    x: f32,
    y: f32,
}

pub struct Gizmo {
    position: Vec2,
    size: f32,
    is_hovered: bool,
}

impl Gizmo {
    pub fn check_hover(self, mouse_pos: Vec2) -> bool {
        self.is_hovered = true
        self.is_hovered
    }
}

pub struct GizmoManager {
    gizmos: Vec<Gizmo>,
}

impl GizmoManager {
    pub fn update_gizmos(self, mouse_pos: Vec2) {
        for i in 0..self.gizmos.len() as i64 {
            self.gizmos[i as usize].check_hover(mouse_pos)
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("editor/viewport.rs").expect("viewport.rs");

    assert!(
        rs.contains("fn update_gizmos(&mut self"),
        "indexed field mutating method call requires &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_self_nested_field_mutating_initialize_needs_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/unified_renderer.wj",
        r#"
pub struct VoxelRenderer {
    ready: bool,
}

impl VoxelRenderer {
    pub fn initialize(self) {
        self.ready = true
    }
}

pub struct MeshRenderer {
    ready: bool,
}

impl MeshRenderer {
    pub fn initialize(self) {
        self.ready = true
    }
}

pub struct UnifiedRenderer {
    voxel_renderer: VoxelRenderer,
    mesh_renderer: MeshRenderer,
}

impl UnifiedRenderer {
    pub fn initialize(self) {
        self.voxel_renderer.initialize()
        self.mesh_renderer.initialize()
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/unified_renderer.rs")
        .expect("unified_renderer.rs");

    assert!(
        rs.contains("fn initialize(&mut self)"),
        "nested field mutating calls require &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_param_field_readonly_method_no_mut_camera_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "math/mat4.wj",
        r#"
pub struct Mat4 {
    m00: f32,
}

impl Mat4 {
    pub fn to_column_major_array(self) -> [f32; 16] {
        [self.m00, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    }

    pub fn inverse(self) -> Mat4 {
        self
    }
}
"#,
    );
    test.add_file(
        "rendering/render_port.wj",
        r#"
use crate::math::mat4::Mat4

pub struct CameraData {
    view_matrix: Mat4,
    proj_matrix: Mat4,
}

pub struct GpuCameraState {
    view_matrix: [f32; 16],
    proj_matrix: [f32; 16],
    inv_view: [f32; 16],
    inv_proj: [f32; 16],
}

pub struct UnifiedRenderer {}

impl UnifiedRenderer {
    fn camera_data_to_gpu_state(camera: CameraData) -> GpuCameraState {
        let view_arr = camera.view_matrix.to_column_major_array()
        let proj_arr = camera.proj_matrix.to_column_major_array()
        let inv_view = camera.view_matrix.inverse().to_column_major_array()
        let inv_proj = camera.proj_matrix.inverse().to_column_major_array()
        GpuCameraState {
            view_matrix: view_arr,
            proj_matrix: proj_arr,
            inv_view: inv_view,
            inv_proj: inv_proj,
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/render_port.rs")
        .expect("render_port.rs");

    assert!(
        !rs.contains("camera: &mut CameraData"),
        "readonly field method chain must not infer &mut param. Got:\n{rs}"
    );
    assert!(
        rs.contains("camera: CameraData") || rs.contains("camera: &CameraData"),
        "expected owned or &CameraData param. Got:\n{rs}"
    );
}

#[test]
fn test_trait_impl_readonly_self_not_upgraded_for_mutating_callee() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "input/input_port.wj",
        r#"
pub trait InputPort {
    fn is_key_pressed(key: i32) -> bool
    fn is_key_just_pressed(key: i32) -> bool
}

pub struct MockInputPort {
    pressed_keys: Vec<i32>,
}

impl MockInputPort {
    pub fn new() -> MockInputPort {
        MockInputPort { pressed_keys: Vec::new() }
    }
}

impl InputPort for MockInputPort {
    fn is_key_pressed(key: i32) -> bool {
        for k in self.pressed_keys {
            if k == key {
                return true
            }
        }
        false
    }

    fn is_key_just_pressed(key: i32) -> bool {
        self.is_key_pressed(key)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("input/input_port.rs").expect("input_port.rs");

    assert!(
        rs.contains("fn is_key_just_pressed(&self"),
        "trait impl must keep &self when trait requires it. Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn is_key_just_pressed(&mut self"),
        "must not upgrade trait impl to &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_direct_initialize_with_early_return_cross_file_needs_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/voxel_gpu_renderer.wj",
        r#"
pub struct VoxelGPURenderer {
    ready: bool,
}

impl VoxelGPURenderer {
    pub fn initialize(self) {
        self.ready = true
    }
}
"#,
    );
    test.add_file(
        "rendering/mesh_renderer.wj",
        r#"
pub struct MeshRenderer {
    ready: bool,
}

impl MeshRenderer {
    pub fn initialize(self) {
        self.ready = true
    }
}
"#,
    );
    test.add_file(
        "rendering/unified_renderer.wj",
        r#"
use crate::rendering::voxel_gpu_renderer::VoxelGPURenderer
use crate::rendering::mesh_renderer::MeshRenderer

pub struct UnifiedRenderer {
    test_mode: bool,
    voxel_renderer: VoxelGPURenderer,
    mesh_renderer: MeshRenderer,
}

impl UnifiedRenderer {
    /// Direct initialize (shadows RenderPort trait for non-trait dispatch)
    pub fn initialize(self) {
        if self.test_mode {
            return
        }
        self.voxel_renderer.initialize()
        self.mesh_renderer.initialize()
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/unified_renderer.rs")
        .expect("unified_renderer.rs");

    assert!(
        rs.contains("pub fn initialize(&mut self)"),
        "cross-file nested mutating calls require &mut self on direct initialize. Got:\n{rs}"
    );
    assert!(
        !rs.contains("pub fn initialize(&self)"),
        "must not emit &self when nested field methods mutate. Got:\n{rs}"
    );
}

#[test]
fn test_trait_impl_mut_receiver_matches_trait_contract() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/render_port.wj",
        r#"
pub trait RenderPort {
    fn initialize()
    fn set_camera(camera: i32)
}

pub struct MockRenderer {
    ready: bool,
    camera_set: bool,
}

impl MockRenderer {
    pub fn new() -> MockRenderer {
        MockRenderer { ready: false, camera_set: false }
    }
}

impl RenderPort for MockRenderer {
    fn initialize() {
    }

    fn set_camera(camera: i32) {
        self.camera_set = true
    }
}
"#,
    );
    test.add_file(
        "rendering/unified_renderer.wj",
        r#"
use crate::rendering::render_port::RenderPort

pub struct VoxelRenderer {
    ready: bool,
}

impl VoxelRenderer {
    pub fn initialize(self) {
        self.ready = true
    }
}

pub struct UnifiedRenderer {
    voxel_renderer: VoxelRenderer,
    ready: bool,
}

impl UnifiedRenderer {
    pub fn new() -> UnifiedRenderer {
        UnifiedRenderer {
            voxel_renderer: VoxelRenderer { ready: false },
            ready: false,
        }
    }
}

impl RenderPort for UnifiedRenderer {
    fn initialize() {
        self.voxel_renderer.initialize()
        self.ready = true
    }

    fn set_camera(camera: i32) {
        self.ready = false
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/unified_renderer.rs")
        .expect("unified_renderer.rs");

    assert!(
        rs.contains("impl RenderPort for UnifiedRenderer")
            && rs.contains("fn initialize(&mut self)"),
        "trait impl must use &mut self when trait contract requires it. Got:\n{rs}"
    );
    assert!(
        !rs.contains("impl RenderPort for UnifiedRenderer")
            || !rs.contains("fn initialize(&self)"),
        "must not downgrade trait impl to &self when trait uses &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_implicit_return_vec_index_no_ref_before_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ecs/scene.wj",
        r#"
pub struct Scene {
    children: Vec<Vec<i64>>,
}

pub struct SceneGraph {
    children: Vec<Vec<i64>>,
}

impl SceneGraph {
    pub fn get_children(self, entity_id: i64) -> Vec<i64> {
        let pi = 0
        self.children[pi as usize]
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ecs/scene.rs").expect("scene.rs");

    assert!(
        !rs.contains("&self.children[") || rs.contains("(&self.children["),
        "implicit return must not emit &vec[i].clone() — use ( &vec[i] ).clone(). Got:\n{rs}"
    );
}

#[test]
fn test_hashmap_get_option_binding_borrows_not_clones() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "animation/controller.wj",
        r#"
pub struct Animation {
    frames: Vec<i32>,
}

impl Animation {
    pub fn get_frame(self, index: usize) -> Option<i32> {
        None
    }
}

pub struct AnimationController {
    animations: Map<string, Animation>,
    current_animation: Option<string>,
    current_frame_index: usize,
}

impl AnimationController {
    pub fn current_frame(self) -> usize {
        if let Some(anim_name) = self.current_animation {
            if let Some(animation) = self.animations.get(anim_name) {
                if let Some(frame) = animation.get_frame(self.current_frame_index) {
                    return frame
                }
            }
        }
        0
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("animation/controller.rs").expect("controller.rs");

    assert!(
        !rs.contains("anim_name.clone()"),
        "HashMap get key must borrow, not clone Option binding. Got:\n{rs}"
    );
}

#[test]
fn test_copy_type_param_not_mut_ref_state_id() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "state_machine/state_id.wj",
        r#"
pub struct StateId {
    value: u32,
}
"#,
    );
    test.add_file(
        "state_machine/machine.wj",
        r#"
use crate::state_machine::state_id::StateId

pub struct State {
    enter_count: i32,
}

impl State {
    pub fn increment_enter_count(self) {
        self.enter_count = self.enter_count + 1
    }
}

pub struct StateMachine {
    states: Map<StateId, State>,
    initial_state: Option<StateId>,
    current_state: Option<StateId>,
    time_in_state: f32,
}

impl StateMachine {
    pub fn set_initial_state(self, id: StateId) {
        if self.states.contains_key(id) {
            self.initial_state = Some(id)
            self.current_state = Some(id)
            if let Some(state) = self.states.get_mut(id) {
                state.increment_enter_count()
            }
        }
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("state_machine/machine.rs").expect("machine.rs");

    assert!(
        !rs.contains("id: &mut StateId"),
        "Copy lookup key params must not become &mut. Got:\n{rs}"
    );
}

#[test]
fn test_hashset_contains_borrows_owned_string_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "scene/manager.wj",
        r#"
pub struct SceneManager {
    registered_scenes: Vec<string>,
}

impl SceneManager {
    pub fn is_registered(self, name: string) -> bool {
        self.registered_scenes.contains(name)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("scene/manager.rs").expect("manager.rs");

    assert!(
        rs.contains("contains(&name)")
            || rs.contains("contains(name.as_str())")
            || (rs.contains("contains(name)")
                && (rs.contains("name: &String") || rs.contains("name: &str"))),
        "Vec/Set contains must borrow string param without to_string(). Got:\n{rs}"
    );
    assert!(
        !rs.contains("contains(name.to_string())"),
        "must not allocate to_string for contains lookup. Got:\n{rs}"
    );
}

#[test]
fn test_set_bool_string_literal_not_to_string_for_str_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "behavior_tree/blackboard.wj",
        r#"
pub struct Blackboard {}

impl Blackboard {
    pub fn set_bool(self, key: string, value: bool) {
        let _ = key
        let _ = value
    }
}
"#,
    );
    test.add_file(
        "enemy/bt_actions.wj",
        r#"
use crate::behavior_tree::blackboard::Blackboard

pub fn populate_conditions(bb: Blackboard) {
    bb.set_bool("__cond_is_alive", true)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("enemy/bt_actions.rs").expect("bt_actions.rs");

    assert!(
        rs.contains("set_bool(\"__cond_is_alive\""),
        "string literal key must stay bare for &str param. Got:\n{rs}"
    );
    assert!(
        !rs.contains("\"__cond_is_alive\".to_string()"),
        "must not allocate to_string for &str key param. Got:\n{rs}"
    );
}

#[test]
fn test_set_name_string_literal_to_string_for_owned_string_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/material.wj",
        r#"
pub struct Material {
    name: string,
}

impl Material {
    pub fn new() -> Material {
        Material { name: "" }
    }

    pub fn set_name(self, name: string) {
        self.name = name
    }
}

pub fn create_metal_material() -> Material {
    let mut material = Material::new()
    material.set_name("Metal")
    material
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("rendering/material.rs").expect("material.rs");

    assert!(
        rs.contains("set_name(\"Metal\".to_string())") || rs.contains("set_name(\"Metal\".into())"),
        "owned String param needs allocation from literal. Got:\n{rs}"
    );
}

#[test]
fn test_add_condition_string_literal_not_to_string_for_str_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "behavior_tree/behavior_tree.wj",
        r#"
pub struct NodeId {
    id: i32,
}

pub struct BehaviorTree {
    next_id: i32,
}

impl BehaviorTree {
    pub fn new() -> BehaviorTree {
        BehaviorTree { next_id: 0 }
    }

    pub fn add_condition(self, name: string) -> NodeId {
        NodeId { id: 0 }
    }
}

pub fn build_tree() -> BehaviorTree {
    let mut tree = BehaviorTree::new()
    let _ = tree.add_condition("is_enemy_near")
    tree
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("behavior_tree/behavior_tree.rs")
        .expect("behavior_tree.rs");

    assert!(
        rs.contains("add_condition(\"is_enemy_near\""),
        "lookup-key condition name must stay bare for &str param. Got:\n{rs}"
    );
    assert!(
        !rs.contains("\"is_enemy_near\".to_string()"),
        "must not allocate to_string for add_condition key. Got:\n{rs}"
    );
}

#[test]
fn test_user_contains_f32_params_not_collection_key_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "audio/reverb_zones.wj",
        r#"
pub struct ReverbZone {
    min_x: f32,
    max_x: f32,
}

impl ReverbZone {
    pub fn contains(self, x: f32, y: f32, z: f32) -> bool {
        x >= self.min_x && x <= self.max_x
    }
}

pub fn test_reverb_zone_contains() {
    let zone = ReverbZone { min_x: 0.0, max_x: 10.0 }
    assert(!zone.contains(-1.0, 2.5, 5.0), "outside")
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("audio/reverb_zones.rs").expect("reverb_zones.rs");

    assert!(
        rs.contains("zone.contains(-1.0_f32") || rs.contains("zone.contains(-1.0,"),
        "f32 contains args pass by value. Got:\n{rs}"
    );
    assert!(
        !rs.contains("contains(&-1.0"),
        "must not borrow Copy f32 for user contains(). Got:\n{rs}"
    );
}

#[test]
fn test_user_get_copy_key_not_hashmap_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "voxel/material.wj",
        r#"
pub struct VoxelMaterial {
    color_r: f32,
}

pub struct MaterialPalette {
    materials: Vec<VoxelMaterial>,
}

impl MaterialPalette {
    pub fn get(self, id: u8) -> VoxelMaterial {
        self.materials[id as usize].clone()
    }

    pub fn validate(self, wall_id: u8) -> i32 {
        let m = self.get(wall_id)
        if m.color_r > 0.5 {
            return 1
        }
        0
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("voxel/material.rs").expect("material.rs");

    assert!(
        rs.contains("self.get(wall_id)") || rs.contains("self.get(wall_id.clone())"),
        "u8 get key passes by value for user get(). Got:\n{rs}"
    );
    assert!(
        !rs.contains("self.get(&wall_id)"),
        "must not borrow Copy u8 for user get(). Got:\n{rs}"
    );
}

#[test]
fn test_caller_above_callee_in_source_needs_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "visual_scripting/graph.wj",
        r#"
pub struct GraphNode {
    value: i32,
}

impl GraphNode {
    pub fn execute(self) {
        self.value = 1
    }
}

pub struct ScriptGraph {
    nodes: Vec<GraphNode>,
}

impl ScriptGraph {
    pub fn execute_from_event(self, _event_id: u32) {
        self.execute_node(0)
    }

    fn execute_node(self, node_idx: usize) {
        self.nodes[node_idx].execute()
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("visual_scripting/graph.rs").expect("graph.rs");

    assert!(
        rs.contains("fn execute_from_event(&mut self"),
        "caller above callee in source must get &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_cross_file_nested_initialize_needs_mut_self() {
    let mut test = MultiFileTest::new();
    // Alphabetical order: unified_renderer before voxel_renderer — simulates engine compile order.
    test.add_file(
        "rendering/unified_renderer.wj",
        r#"
use crate::rendering::voxel_renderer::VoxelRenderer

pub struct UnifiedRenderer {
    voxel_renderer: VoxelRenderer,
}

impl UnifiedRenderer {
    pub fn initialize(self) {
        self.voxel_renderer.initialize()
    }
}
"#,
    );
    test.add_file(
        "rendering/voxel_renderer.wj",
        r#"
pub struct VoxelRenderer {
    ready: bool,
}

impl VoxelRenderer {
    pub fn initialize(self) {
        self.ready = true
    }
}
"#,
    );
    test.add_file(
        "rendering/mod.wj",
        r#"
pub mod unified_renderer
pub mod voxel_renderer
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("rendering/unified_renderer.rs")
        .expect("unified_renderer.rs");

    assert!(
        rs.contains("fn initialize(&mut self)"),
        "cross-file nested initialize requires &mut self. Got:\n{rs}"
    );
}

#[test]
fn test_hashset_contains_no_double_borrow_on_borrowed_string_param() {
    // Dogfooding: scene/manager.wj — `name: &String` + `contains(&name)` → E0277.
    let mut test = MultiFileTest::new();
    test.add_file(
        "scene/manager.wj",
        r#"
use std::collections::HashSet

pub struct SceneManager {
    registered_scenes: HashSet<string>,
    paused_scenes: HashSet<string>,
}

impl SceneManager {
    pub fn is_registered(self, name: string) -> bool {
        self.registered_scenes.contains(name)
    }

    pub fn is_paused(self, name: string) -> bool {
        self.paused_scenes.contains(name)
    }
}
"#,
    );

    let map = test
        .compile()
        .unwrap_or_else(|e| panic!("compile failed: {e}"));
    let rs = map.get("scene/manager.rs").expect("manager.rs");

    assert!(
        !rs.contains("contains(&name)"),
        "borrowed string param must not double-borrow for HashSet::contains. Got:\n{rs}"
    );
    assert!(
        rs.contains("contains(name)"),
        "HashSet::contains should pass borrowed param by value. Got:\n{rs}"
    );
}

#[test]
fn test_qualified_new_i32_literal_not_borrowed_with_homonym_new_registry() {
    // Multipass registry has thousands of `::new` entries; must not borrow `64` for i32 params.
    let mut test = MultiFileTest::new();
    test.add_file(
        "other/type_a.wj",
        r#"
pub struct TypeA {}

impl TypeA {
    pub fn new() -> TypeA {
        TypeA {}
    }
}
"#,
    );
    test.add_file(
        "quick_start/voxel_scene.wj",
        r#"
pub struct VoxelScene {}

impl VoxelScene {
    pub fn new(grid_size: i32) -> VoxelScene {
        VoxelScene {}
    }
}
"#,
    );
    test.add_file(
        "quick_start/game.wj",
        r#"
use crate::quick_start::voxel_scene::VoxelScene

pub fn quick_start() -> VoxelScene {
    VoxelScene::new(64)
}
"#,
    );

    let map = test
        .compile()
        .unwrap_or_else(|e| panic!("compile failed: {e}"));
    let rs = map.get("quick_start/game.rs").expect("game.rs");

    assert!(
        (rs.contains("VoxelScene::new(64)") || rs.contains("VoxelScene::new(64_i32)"))
            && !rs.contains("VoxelScene::new(&64)"),
        "i32 constructor literal must not be auto-borrowed. Got:\n{rs}"
    );
}

#[test]
fn test_qualified_new_i32_literal_with_stale_voxelscene_metadata_homonym() {
    use std::fs;
    use windjammer::{build_project_ext, CompilationTarget};

    let mut test = MultiFileTest::new();
    test.add_file(
        "other/type_a.wj",
        r#"
pub struct TypeA {}

impl TypeA {
    pub fn new() -> TypeA {
        TypeA {}
    }
}
"#,
    );
    test.add_file(
        "quick_start/voxel_scene.wj",
        r#"
pub struct VoxelScene {}

impl VoxelScene {
    pub fn new(grid_size: i32) -> VoxelScene {
        VoxelScene {}
    }
}
"#,
    );
    test.add_file(
        "quick_start/game.wj",
        r#"
use crate::quick_start::voxel_scene::VoxelScene

pub fn quick_start() -> VoxelScene {
    VoxelScene::new(64)
}
"#,
    );

    let meta_path = test.build_dir().parent().unwrap().join("stale_metadata.json");
    fs::write(
        &meta_path,
        r#"{
  "functions": {
    "VoxelScene::new": {
      "params": [],
      "return_type": "Custom(\"VoxelScene\")",
      "is_associated": true,
      "parent_type": "VoxelScene",
      "param_ownership": [],
      "has_self_receiver": false,
      "is_extern": false
    },
    "material::new": {
      "params": ["Custom(\"MaterialPalette\")"],
      "return_type": "Custom(\"Material\")",
      "is_associated": false,
      "parent_type": null,
      "param_ownership": ["Borrowed"],
      "has_self_receiver": false,
      "is_extern": false
    }
  }
}"#,
    )
    .expect("write stale metadata");

    build_project_ext(
        &test.build_dir().parent().unwrap().join("src"),
        test.build_dir(),
        CompilationTarget::Rust,
        false,
        true,
        &[("game_core", &meta_path)],
    )
    .unwrap_or_else(|e| panic!("compile with stale metadata failed: {e}"));

    let rs = fs::read_to_string(test.build_dir().join("quick_start/game.rs")).expect("game.rs");
    assert!(
        (rs.contains("VoxelScene::new(64)") || rs.contains("VoxelScene::new(64_i32)"))
            && !rs.contains("VoxelScene::new(&64)"),
        "stale 0-arg VoxelScene::new metadata must not borrow i32 literal. Got:\n{rs}"
    );
}

#[test]
fn test_qualified_new_i32_literal_when_game_file_is_analyzed_before_voxel_scene() {
    // Engine discovery order is alphabetical: `quick_start/game.wj` before
    // `quick_start/voxel_scene.wj`. Codegen must not borrow `64` while waiting
    // for the defining module's constructor signature.
    let mut test = MultiFileTest::new();
    test.add_file(
        "quick_start/game.wj",
        r#"
use crate::quick_start::voxel_scene::VoxelScene

pub fn quick_start() -> VoxelScene {
    VoxelScene::new(64)
}
"#,
    );
    test.add_file(
        "quick_start/voxel_scene.wj",
        r#"
pub struct VoxelScene {}

impl VoxelScene {
    pub fn new(grid_size: i32) -> VoxelScene {
        VoxelScene {}
    }
}
"#,
    );

    let map = test
        .compile()
        .unwrap_or_else(|e| panic!("compile failed: {e}"));
    let rs = map.get("quick_start/game.rs").expect("game.rs");

    assert!(
        (rs.contains("VoxelScene::new(64)") || rs.contains("VoxelScene::new(64_i32)"))
            && !rs.contains("VoxelScene::new(&64)"),
        "game.wj before voxel_scene.wj must not auto-borrow i32 ctor literal. Got:\n{rs}"
    );
}

#[test]
fn test_qualified_new_i32_literal_game_before_voxel_scene_with_stale_metadata() {
    use std::fs;
    use windjammer::{build_project_ext, CompilationTarget};

    let mut test = MultiFileTest::new();
    test.add_file(
        "quick_start/game.wj",
        r#"
use crate::quick_start::voxel_scene::VoxelScene

pub fn quick_start() -> VoxelScene {
    VoxelScene::new(64)
}
"#,
    );
    test.add_file(
        "quick_start/voxel_scene.wj",
        r#"
pub struct VoxelScene {}

impl VoxelScene {
    pub fn new(grid_size: i32) -> VoxelScene {
        VoxelScene {}
    }
}
"#,
    );

    let meta_path = test.build_dir().parent().unwrap().join("stale_metadata.json");
    fs::write(
        &meta_path,
        r#"{
  "functions": {
    "VoxelScene::new": {
      "params": [],
      "return_type": "Custom(\"VoxelScene\")",
      "is_associated": true,
      "parent_type": "VoxelScene",
      "param_ownership": [],
      "has_self_receiver": false,
      "is_extern": false
    },
    "quick_start::voxel_scene::VoxelScene::new": {
      "params": ["Custom(\"i32\")"],
      "return_type": "Custom(\"VoxelScene\")",
      "is_associated": false,
      "parent_type": null,
      "param_ownership": ["Owned"],
      "has_self_receiver": false,
      "is_extern": false
    },
    "material::new": {
      "params": ["Custom(\"MaterialPalette\")"],
      "return_type": "Custom(\"Material\")",
      "is_associated": false,
      "parent_type": null,
      "param_ownership": ["Borrowed"],
      "has_self_receiver": false,
      "is_extern": false
    }
  }
}"#,
    )
    .expect("write stale metadata");

    build_project_ext(
        &test.build_dir().parent().unwrap().join("src"),
        test.build_dir(),
        CompilationTarget::Rust,
        false,
        true,
        &[("game_core", &meta_path)],
    )
    .unwrap_or_else(|e| panic!("compile failed: {e}"));

    let rs = fs::read_to_string(test.build_dir().join("quick_start/game.rs")).expect("game.rs");
    assert!(
        (rs.contains("VoxelScene::new(64)") || rs.contains("VoxelScene::new(64_i32)"))
            && !rs.contains("VoxelScene::new(&64)"),
        "stale metadata + game-before-voxel must not borrow i32 literal. Got:\n{rs}"
    );
}

#[test]
fn test_homonymous_voxel_scene_modules_game_before_defs_with_metadata() {
    use std::fs;
    use windjammer::{build_project_ext, CompilationTarget};

    let mut test = MultiFileTest::new();
    test.add_file(
        "quick_start/game.wj",
        r#"
use crate::quick_start::voxel_scene::VoxelScene

pub fn quick_start() -> VoxelScene {
    VoxelScene::new(64)
}
"#,
    );
    test.add_file(
        "quick_start/voxel_scene.wj",
        r#"
pub struct VoxelScene {}

impl VoxelScene {
    pub fn new(grid_size: i32) -> VoxelScene {
        VoxelScene {}
    }
}
"#,
    );
    test.add_file(
        "voxel/voxel_scene.wj",
        r#"
pub struct VoxelScene {}

impl VoxelScene {
    pub fn new() -> VoxelScene {
        VoxelScene {}
    }
}
"#,
    );

    let meta_path = test.build_dir().parent().unwrap().join("stale_metadata.json");
    fs::write(
        &meta_path,
        r#"{
  "functions": {
    "VoxelScene::new": {
      "params": [],
      "return_type": "Custom(\"VoxelScene\")",
      "is_associated": true,
      "parent_type": "VoxelScene",
      "param_ownership": [],
      "has_self_receiver": false,
      "is_extern": false
    },
    "quick_start::voxel_scene::VoxelScene::new": {
      "params": ["Custom(\"i32\")"],
      "return_type": "Custom(\"VoxelScene\")",
      "is_associated": false,
      "parent_type": null,
      "param_ownership": ["Owned"],
      "has_self_receiver": false,
      "is_extern": false
    },
    "voxel::voxel_scene::VoxelScene::new": {
      "params": [],
      "return_type": "Custom(\"VoxelScene\")",
      "is_associated": false,
      "parent_type": null,
      "param_ownership": [],
      "has_self_receiver": false,
      "is_extern": false
    }
  }
}"#,
    )
    .expect("write metadata");

    build_project_ext(
        &test.build_dir().parent().unwrap().join("src"),
        test.build_dir(),
        CompilationTarget::Rust,
        false,
        true,
        &[("game_core", &meta_path)],
    )
    .unwrap_or_else(|e| panic!("compile failed: {e}"));

    let rs = fs::read_to_string(test.build_dir().join("quick_start/game.rs")).expect("game.rs");
    assert!(
        (rs.contains("VoxelScene::new(64)") || rs.contains("VoxelScene::new(64_i32)"))
            && !rs.contains("VoxelScene::new(&64)"),
        "homonymous VoxelScene modules must not borrow i32 literal. Got:\n{rs}"
    );
}

#[test]
fn test_item_new_u32_id_not_borrowed_with_stale_str_metadata() {
    use std::fs;
    use windjammer::{build_project_ext, CompilationTarget};

    let mut test = MultiFileTest::new();
    test.add_file(
        "inventory/equipment.wj",
        r#"
use crate::inventory::item::Item

pub enum EquipSlot {
    Weapon,
}

pub struct EquipmentStats {
    damage_bonus: f32,
}

pub struct EquippableItem {
    item: Item,
    slot: EquipSlot,
    stats: EquipmentStats,
}

impl EquippableItem {
    pub fn weapon(id: u32, name: string, description: string, damage: f32) -> EquippableItem {
        EquippableItem {
            item: Item::new(id, name, description),
            slot: EquipSlot::Weapon,
            stats: EquipmentStats { damage_bonus: damage },
        }
    }
}
"#,
    );
    test.add_file(
        "inventory/item.wj",
        r#"
pub struct Item {
    id: u32,
    name: string,
    description: string,
}

impl Item {
    pub fn new(id: u32, name: string, description: string) -> Item {
        Item { id: id, name: name, description: description }
    }
}
"#,
    );

    let meta_path = test.build_dir().parent().unwrap().join("stale_item_metadata.json");
    fs::write(
        &meta_path,
        r#"{
  "functions": {
    "Item::new": {
      "params": [
        "Reference(Custom(\"str\"))",
        "Reference(Custom(\"str\"))",
        "Reference(Custom(\"str\"))"
      ],
      "return_type": "Custom(\"Item\")",
      "is_associated": true,
      "parent_type": "Item",
      "param_ownership": ["Borrowed", "Borrowed", "Borrowed"],
      "has_self_receiver": false,
      "is_extern": false
    }
  }
}"#,
    )
    .expect("write stale metadata");

    build_project_ext(
        &test.build_dir().parent().unwrap().join("src"),
        test.build_dir(),
        CompilationTarget::Rust,
        false,
        true,
        &[("game_core", &meta_path)],
    )
    .unwrap_or_else(|e| panic!("compile with stale Item metadata failed: {e}"));

    let rs = fs::read_to_string(test.build_dir().join("inventory/equipment.rs")).expect("equipment.rs");
    assert!(
        rs.contains("Item::new(id,") && !rs.contains("Item::new(&id,"),
        "Copy u32 id must not be auto-borrowed even when stale metadata marks all params Borrowed str. Got:\n{rs}"
    );
}

#[test]
fn test_item_new_u32_not_borrowed_when_rpg_item_homonym_exists() {
    use std::fs;
    use windjammer::{build_project_ext, CompilationTarget};

    let mut test = MultiFileTest::new();
    test.add_file(
        "inventory/equipment.wj",
        r#"
use crate::inventory::item::Item

pub enum EquipSlot {
    Weapon,
}

pub struct EquipmentStats {
    damage_bonus: f32,
}

pub struct EquippableItem {
    item: Item,
    slot: EquipSlot,
    stats: EquipmentStats,
}

impl EquippableItem {
    pub fn weapon(id: u32, name: string, description: string, damage: f32) -> EquippableItem {
        EquippableItem {
            item: Item::new(id, name, description),
            slot: EquipSlot::Weapon,
            stats: EquipmentStats { damage_bonus: damage },
        }
    }
}
"#,
    );
    test.add_file(
        "inventory/item.wj",
        r#"
pub struct Item {
    id: u32,
    name: string,
    description: string,
}

impl Item {
    pub fn new(id: u32, name: string, description: string) -> Item {
        Item { id: id, name: name, description: description }
    }
}
"#,
    );
    test.add_file(
        "rpg/item.wj",
        r#"
pub struct Item {
    id: string,
    name: string,
    description: string,
}

impl Item {
    pub fn new(id: string, name: string, description: string) -> Item {
        Item { id: id, name: name, description: description }
    }
}
"#,
    );

    build_project_ext(
        &test.build_dir().parent().unwrap().join("src"),
        test.build_dir(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .unwrap_or_else(|e| panic!("compile with homonymous Item types failed: {e}"));

    let rs = fs::read_to_string(test.build_dir().join("inventory/equipment.rs")).expect("equipment.rs");
    assert!(
        rs.contains("Item::new(id,") && !rs.contains("Item::new(&id,"),
        "inventory Item::new(u32) must not borrow id when rpg Item homonym exists. Got:\n{rs}"
    );
}

#[test]
fn test_string_literal_not_to_string_for_borrowed_str_param_in_multipass() {
    use std::fs;
    use windjammer::{build_project_ext, CompilationTarget};

    let mut test = MultiFileTest::new();
    test.add_file(
        "bots/metrics_collector.wj",
        r#"
pub struct PlaytestMetrics {
    samples: Vec<f32>,
}

impl PlaytestMetrics {
    pub fn new() -> PlaytestMetrics {
        PlaytestMetrics { samples: Vec::new() }
    }

    pub fn record_section_frame_ms(self, section: string, ms: f32) {
        let _ = section
        let _ = ms
    }
}
"#,
    );
    test.add_file(
        "bots/playtest_hooks.wj",
        r#"
use crate::bots::metrics_collector::PlaytestMetrics

pub fn smoke() {
    let mut m = PlaytestMetrics::new()
    m.record_section_frame_ms("section_a", 10.0)
}
"#,
    );

    build_project_ext(
        &test.build_dir().parent().unwrap().join("src"),
        test.build_dir(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .unwrap_or_else(|e| panic!("compile failed: {e}"));

    let rs = fs::read_to_string(test.build_dir().join("bots/playtest_hooks.rs")).expect("playtest_hooks.rs");
    assert!(
        rs.contains("record_section_frame_ms(\"section_a\""),
        "borrowed string param must not allocate to_string on literal. Got:\n{rs}"
    );
    assert!(
        !rs.contains("\"section_a\".to_string()"),
        "must not use to_string for &str param before metrics module converges. Got:\n{rs}"
    );
}

#[test]
fn test_borrowed_fn_param_not_cloned_for_nested_method_call() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "dialogue/system.wj",
        r#"
pub struct DialogueState {
    honor: i32,
    flags: Vec<string>,
}

impl DialogueState {
    pub fn new() -> DialogueState {
        DialogueState { honor: 0, flags: Vec::new() }
    }

    pub fn is_flag_set(self, flag: string) -> bool {
        true
    }
}

pub enum DialogueCondition {
    FlagSet(string),
}

impl DialogueCondition {
    pub fn is_met(self, state: DialogueState) -> bool {
        match self {
            DialogueCondition::FlagSet(flag) => state.is_flag_set(flag),
        }
    }
}

pub struct DialogueLine {
    conditions: Vec<DialogueCondition>,
}

impl DialogueLine {
    pub fn is_available(self, state: DialogueState) -> bool {
        for i in 0..self.conditions.len() {
            if !self.conditions[i].is_met(state) {
                return false
            }
        }
        true
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("dialogue/system.rs").expect("system.rs");

    assert!(
        rs.contains(".is_met(state)") && !rs.contains(".is_met(state.clone())"),
        "borrowed state param must pass through without clone. Got:\n{rs}"
    );
}

#[test]
fn test_borrowed_struct_param_passes_ref_not_clone_at_method_call() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "bots/pathfinding.wj",
        r#"
pub struct TerrainGrid {
    width: i32,
    depth: i32,
}

impl TerrainGrid {
    pub fn in_bounds(self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.width && z < self.depth
    }
}

pub struct AStarWorkspace {
    closed: Vec<bool>,
}

impl AStarWorkspace {
    pub fn new() -> AStarWorkspace {
        AStarWorkspace { closed: Vec::new() }
    }

    pub fn relax_neighbor(self, grid: TerrainGrid, nx: i32, nz: i32) {
        if !grid.in_bounds(nx, nz) {
            return
        }
        self.closed.push(true)
    }
}

pub fn find_step(grid: TerrainGrid, ws: AStarWorkspace, nx: i32, nz: i32) {
    let mut w = ws
    w.relax_neighbor(grid, nx, nz)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("bots/pathfinding.rs").expect("pathfinding.rs");

    assert!(
        !rs.contains("relax_neighbor(grid.clone()"),
        "borrowed grid param must not clone at call site. Got:\n{rs}"
    );
    assert!(
        rs.contains("w.relax_neighbor(&grid,") || rs.contains("w.relax_neighbor( &grid,")
            || rs.contains("w.relax_neighbor(grid,"),
        "relax_neighbor call must pass grid by ref or compatible borrow. Got:\n{rs}"
    );
}

#[test]
fn test_extern_string_to_ffi_clones_self_field_without_move() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "audio/sound.wj",
        r#"
extern fn audio_play_music(name: string)

pub struct SoundManager {
    current_track: string,
}

impl SoundManager {
    pub fn new() -> SoundManager {
        SoundManager { current_track: "" }
    }

    pub fn replay(self) {
        audio_play_music(self.current_track)
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("audio/sound.rs").expect("sound.rs");

    assert!(
        rs.contains("string_to_ffi(self.current_track.clone())"),
        "extern owned string param must clone self.field to avoid E0507 move. Got:\n{rs}"
    );
    assert!(
        !rs.contains("string_to_ffi(self.current_track))"),
        "must not move self.current_track into extern call. Got:\n{rs}"
    );
}

#[test]
fn test_match_borrow_break_clone_binding_is_mut_for_ref_mut_arm() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "inventory/stack.wj",
        r#"
pub struct Item {
    id: string,
}

pub struct ItemStack {
    item: Item,
    quantity: i32,
}

pub struct Inventory {
    slots: Vec<Option<ItemStack>>,
    capacity: i32,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory { slots: Vec::new(), capacity: 10 }
    }

    pub fn remove_quantity(self, item_id: string, amount: i32) -> bool {
        let mut remaining: i32 = amount
        let mut i: i32 = 0
        while i < self.capacity && remaining > 0 {
            if let Some(stack) = self.slots[i as usize] {
                if stack.item.id == item_id {
                    if stack.quantity <= remaining {
                        remaining = remaining - stack.quantity
                        self.slots[i as usize] = None
                    } else {
                        stack.quantity = stack.quantity - remaining
                        self.slots[i as usize] = Some(stack)
                        remaining = 0
                    }
                }
            }
            i = i + 1
        }
        remaining == 0
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("inventory/stack.rs").expect("stack.rs");

    assert!(
        rs.contains("let mut __match_borrow_break = self.slots[") && rs.contains("].clone();"),
        "clone borrow-break for mut stack binding must use mut binding (E0596). Got:\n{rs}"
    );
}

#[test]
fn test_format_temp_passes_owned_string_for_set_flag_not_ref_string() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "dialogue/flags.wj",
        r#"
pub struct DialogueState {
    flags: Vec<string>,
}

impl DialogueState {
    pub fn set_flag(self, flag: string) {
        self.flags.push(flag)
    }

    pub fn apply_fail(self, quest_name: string) {
        self.set_flag("quest_failed_${quest_name}")
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("dialogue/flags.rs").expect("flags.rs");

    assert!(
        rs.contains("set_flag(_temp0)") || rs.contains("set_flag( _temp0 )"),
        "owned String param must take format temp by value, not &String. Got:\n{rs}"
    );
    assert!(
        !rs.contains("set_flag(&_temp"),
        "must not borrow format temp for owned String param. Got:\n{rs}"
    );
}

#[test]
fn test_global_converged_mut_borrow_preferred_over_stub_owned_at_call() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "voxel/grid_ops.wj",
        r#"
pub struct VoxelGrid {
    data: Vec<i32>,
}

impl VoxelGrid {
    pub fn new() -> VoxelGrid {
        VoxelGrid { data: Vec::new() }
    }

    pub fn mark(self, x: i32) {
        self.data.push(x)
    }
}

pub fn touch_grid(grid: VoxelGrid, x: i32) {
    grid.mark(x)
}
"#,
    );
    test.add_file(
        "voxel/build.wj",
        r#"
use crate::voxel::grid_ops::{VoxelGrid, touch_grid}

pub fn build_grid() {
    let mut grid = VoxelGrid::new()
    touch_grid(grid, 1)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("voxel/build.rs").expect("build.rs");

    assert!(
        rs.contains("touch_grid(&mut grid,") || rs.contains("touch_grid( &mut grid,"),
        "mut-borrow grid param must pass &mut grid, not owned clone. Got:\n{rs}"
    );
    assert!(
        !rs.contains("touch_grid(grid.clone()"),
        "must not clone grid for &mut VoxelGrid param. Got:\n{rs}"
    );
}
