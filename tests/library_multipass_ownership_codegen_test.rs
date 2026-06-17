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
        !rs.contains("fn evaluate(&self,"),
        "evaluate must not take &self when dispatching to evaluate_node. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn evaluate_node(&mut self,")
            || rs.contains("fn evaluate_node(self,")
            || rs.contains("fn evaluate_node(mut self,"),
        "evaluate_node must be callable from evaluate without moving outer &mut self incorrectly. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn evaluate(&mut self,")
            || rs.contains("fn evaluate(self,")
            || rs.contains("fn evaluate(mut self,"),
        "evaluate must use owned or &mut self, not &self. Got:\n{rs}"
    );
    assert!(
        !(rs.contains("fn evaluate(&mut self,") && rs.contains("evaluate_node(self,")),
        "must not mix &mut evaluate with owned evaluate_node call. Got:\n{rs}"
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
        rs.contains("fn evaluate_node(&mut self,"),
        "two recursive self calls require &mut self. Got:\n{rs}"
    );
    assert!(
        !rs.contains("fn evaluate_node(self,"),
        "owned evaluate_node with two recursive calls is invalid Rust. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn evaluate(&mut self,")
            || rs.contains("fn evaluate(self,")
            || rs.contains("fn evaluate(mut self,"),
        "evaluate must not use &self. Got:\n{rs}"
    );
    assert!(
        !(rs.contains("fn evaluate(&mut self,") && rs.contains("evaluate_node(self,")),
        "must not mix &mut evaluate with owned evaluate_node. Got:\n{rs}"
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
        rs.contains("let __match_borrow_break = self.nodes[") && rs.contains("].clone();"),
        "enum borrow-break must clone owned node. Got:\n{rs}"
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

    assert!(
        rs.contains("fn check_collision(a: &RigidBody2D, b: &RigidBody2D)"),
        "match-on-field should infer borrowed params. Got:\n{rs}"
    );
    assert!(
        rs.contains("check_collision(&a, &b)")
            || rs.contains("check_collision(& a, & b)"),
        "borrowed callee params must get & at call site. Got:\n{rs}"
    );
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

    assert!(
        rs.contains("compositor.clone()"),
        "borrowed self returning owned nested field must clone. Got:\n{rs}"
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
