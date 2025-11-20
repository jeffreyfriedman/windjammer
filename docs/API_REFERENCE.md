# Windjammer API Reference

**Complete API documentation for the Windjammer Game Framework**

Version: 0.34.0  
Last Updated: November 20, 2024

---

## Table of Contents

1. [Core](#core)
2. [ECS (Entity Component System)](#ecs)
3. [Rendering](#rendering)
4. [Physics](#physics)
5. [Audio](#audio)
6. [Input](#input)
7. [Networking](#networking)
8. [AI](#ai)
9. [Animation](#animation)
10. [UI](#ui)
11. [Assets](#assets)
12. [Math](#math)
13. [Optimization](#optimization)

---

## Core

### App

Main application entry point.

```rust
pub struct App {
    // ECS world
    // Systems
    // Resources
}

impl App {
    pub fn new() -> Self;
    pub fn add_system<S: System>(&mut self, system: S) -> &mut Self;
    pub fn add_startup_system<S: System>(&mut self, system: S) -> &mut Self;
    pub fn add_shutdown_system<S: System>(&mut self, system: S) -> &mut Self;
    pub fn run(&mut self);
}
```

**Example:**
```rust
let mut app = App::new();
app.add_startup_system(setup);
app.add_system(update);
app.run();
```

### Time

Frame timing and delta time.

```rust
pub struct Time {
    pub delta: f32,           // Time since last frame (seconds)
    pub elapsed: f32,         // Total time since start (seconds)
    pub frame_count: u64,     // Total frames rendered
    pub fps: f32,             // Frames per second
}

impl Time {
    pub fn delta_seconds(&self) -> f32;
    pub fn elapsed_seconds(&self) -> f32;
}
```

---

## ECS

### Entity

Unique identifier for game objects.

```rust
pub struct Entity(u64);

impl Entity {
    pub fn id(&self) -> u64;
}
```

### World

ECS world containing all entities and components.

```rust
pub struct World;

impl World {
    pub fn spawn(&mut self) -> Entity;
    pub fn despawn(&mut self, entity: Entity);
    pub fn add_component<C: Component>(&mut self, entity: Entity, component: C);
    pub fn get_component<C: Component>(&self, entity: Entity) -> Option<&C>;
    pub fn get_component_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C>;
    pub fn remove_component<C: Component>(&mut self, entity: Entity);
}
```

### Query

Query entities by components.

```rust
pub struct Query<'w, Q: QueryData>;

impl<'w, Q: QueryData> Query<'w, Q> {
    pub fn iter(&self) -> impl Iterator<Item = Q::Item<'_>>;
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Q::Item<'_>>;
}
```

**Example:**
```rust
fn update_positions(mut query: Query<(&Transform, &Velocity)>) {
    for (transform, velocity) in query.iter_mut() {
        transform.translation += velocity.0 * time.delta;
    }
}
```

---

## Rendering

### Camera3D

3D perspective camera.

```rust
pub struct Camera3D {
    pub position: Vec3,
    pub look_at: Vec3,
    pub up: Vec3,
    pub fov: f32,              // Field of view (degrees)
    pub near: f32,             // Near clipping plane
    pub far: f32,              // Far clipping plane
}

impl Camera3D {
    pub fn new(position: Vec3, look_at: Vec3, fov: f32) -> Self;
    pub fn view_matrix(&self) -> Mat4;
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4;
}
```

### Camera2D

2D orthographic camera.

```rust
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
}

impl Camera2D {
    pub fn new(position: Vec2, zoom: f32) -> Self;
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2;
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2;
}
```

### Mesh

3D mesh data.

```rust
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bounding_box: AABB,
}

impl Mesh {
    pub fn cube(size: f32) -> Self;
    pub fn sphere(radius: f32, subdivisions: u32) -> Self;
    pub fn plane(size: f32) -> Self;
    pub fn cylinder(radius: f32, height: f32, subdivisions: u32) -> Self;
    pub fn from_obj(path: &str) -> Result<Self, AssetError>;
}
```

### Material

PBR material properties.

```rust
pub struct Material {
    pub albedo: Color,
    pub metallic: f32,         // 0.0 = dielectric, 1.0 = metal
    pub roughness: f32,        // 0.0 = smooth, 1.0 = rough
    pub emissive: Color,       // Emissive color for bloom
    pub albedo_texture: Option<Handle<Texture>>,
    pub normal_texture: Option<Handle<Texture>>,
    pub metallic_roughness_texture: Option<Handle<Texture>>,
}
```

### PointLight

Omnidirectional light source.

```rust
pub struct PointLight {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub cast_shadows: bool,
}
```

### DirectionalLight

Directional light (sun).

```rust
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub cast_shadows: bool,
}
```

### SpotLight

Cone-shaped light source.

```rust
pub struct SpotLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub inner_angle: f32,      // Inner cone angle (degrees)
    pub outer_angle: f32,      // Outer cone angle (degrees)
    pub cast_shadows: bool,
}
```

### PostProcessing

Post-processing effects.

```rust
pub struct PostProcessing {
    pub hdr_enabled: bool,
    pub bloom: Option<BloomSettings>,
    pub ssao: Option<SSAOSettings>,
    pub tone_mapping: ToneMappingMode,
    pub exposure: f32,
    pub color_grading: ColorGrading,
}

pub struct BloomSettings {
    pub threshold: f32,        // Brightness threshold
    pub intensity: f32,        // Bloom strength
    pub radius: f32,           // Bloom spread
    pub soft_knee: f32,        // Smooth transition
}

pub struct SSAOSettings {
    pub radius: f32,           // Sample radius
    pub intensity: f32,        // Effect strength
    pub bias: f32,             // Depth bias
    pub samples: u32,          // Quality (8-64)
}

pub enum ToneMappingMode {
    None,
    Reinhard,
    ACES,                      // Filmic (recommended)
    Uncharted2,
}

pub struct ColorGrading {
    pub temperature: f32,      // -1.0 to 1.0 (cool to warm)
    pub tint: f32,             // -1.0 to 1.0 (green to magenta)
    pub saturation: f32,       // 0.0 to 2.0
    pub contrast: f32,         // 0.0 to 2.0
}
```

---

## Physics

### RigidBody

Physics rigid body.

```rust
pub struct RigidBody {
    pub body_type: RigidBodyType,
    pub mass: f32,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
}

pub enum RigidBodyType {
    Dynamic,                   // Affected by forces
    Static,                    // Never moves
    Kinematic,                 // Moved programmatically
}
```

### Collider

Physics collision shape.

```rust
pub struct Collider {
    pub shape: ColliderShape,
    pub friction: f32,
    pub restitution: f32,      // Bounciness
    pub is_sensor: bool,       // Trigger volume
}

pub enum ColliderShape {
    Box { half_extents: Vec3 },
    Sphere { radius: f32 },
    Capsule { radius: f32, half_height: f32 },
    Cylinder { radius: f32, half_height: f32 },
    Mesh { mesh: Handle<Mesh> },
}
```

### CharacterController

Character movement controller.

```rust
pub struct CharacterController {
    pub move_speed: f32,
    pub jump_force: f32,
    pub gravity: f32,
    pub is_grounded: bool,
}

impl CharacterController {
    pub fn move_character(&mut self, direction: Vec3, delta: f32);
    pub fn jump(&mut self);
}
```

---

## Audio

### AudioSource

3D audio source.

```rust
pub struct AudioSource {
    pub clip: Handle<AudioClip>,
    pub volume: f32,           // 0.0 to 1.0
    pub pitch: f32,            // 0.5 to 2.0
    pub looping: bool,
    pub spatial: bool,         // 3D spatial audio
    pub min_distance: f32,     // Full volume distance
    pub max_distance: f32,     // Silent distance
    pub doppler_factor: f32,   // Doppler effect strength
}

impl AudioSource {
    pub fn play(&mut self);
    pub fn pause(&mut self);
    pub fn stop(&mut self);
    pub fn is_playing(&self) -> bool;
}
```

### AudioBus

Audio mixing bus.

```rust
pub struct AudioBus {
    pub name: String,
    pub volume: f32,
    pub parent: Option<Handle<AudioBus>>,
    pub effects: Vec<AudioEffect>,
}

pub enum AudioEffect {
    Reverb { room_size: f32, damping: f32, wet: f32 },
    Echo { delay: f32, feedback: f32, wet: f32 },
    LowPassFilter { cutoff: f32, resonance: f32 },
    HighPassFilter { cutoff: f32, resonance: f32 },
    Distortion { drive: f32, wet: f32 },
    Chorus { rate: f32, depth: f32, wet: f32 },
}
```

---

## Input

### Input

Input state query.

```rust
pub struct Input;

impl Input {
    // Keyboard
    pub fn key_pressed(key: KeyCode) -> bool;
    pub fn key_just_pressed(key: KeyCode) -> bool;
    pub fn key_just_released(key: KeyCode) -> bool;
    
    // Mouse
    pub fn mouse_button_pressed(button: MouseButton) -> bool;
    pub fn mouse_button_just_pressed(button: MouseButton) -> bool;
    pub fn mouse_button_just_released(button: MouseButton) -> bool;
    pub fn mouse_position() -> Vec2;
    pub fn mouse_delta() -> Vec2;
    pub fn mouse_wheel_delta() -> f32;
    
    // Gamepad
    pub fn gamepad_button_pressed(gamepad: u32, button: GamepadButton) -> bool;
    pub fn gamepad_axis(gamepad: u32, axis: GamepadAxis) -> f32;
}

pub enum KeyCode {
    W, A, S, D, Space, Shift, Ctrl, Alt,
    // ... full keyboard
}

pub enum MouseButton {
    Left, Right, Middle,
}

pub enum GamepadButton {
    South, East, North, West,  // A/B/X/Y or Cross/Circle/Square/Triangle
    LeftBumper, RightBumper,
    LeftTrigger, RightTrigger,
    // ...
}
```

---

## Networking

### NetworkClient

Client-side networking.

```rust
pub struct NetworkClient;

impl NetworkClient {
    pub fn connect(address: &str) -> Result<Self, NetworkError>;
    pub fn disconnect(&mut self);
    pub fn send_message<T: Serialize>(&mut self, message: &T, channel: Channel);
    pub fn receive_messages<T: DeserializeOwned>(&mut self) -> Vec<T>;
    pub fn is_connected(&self) -> bool;
}

pub enum Channel {
    Reliable,                  // TCP-like
    Unreliable,                // UDP-like
}
```

### NetworkServer

Server-side networking.

```rust
pub struct NetworkServer;

impl NetworkServer {
    pub fn start(port: u16) -> Result<Self, NetworkError>;
    pub fn stop(&mut self);
    pub fn send_to_client<T: Serialize>(&mut self, client_id: u64, message: &T, channel: Channel);
    pub fn broadcast<T: Serialize>(&mut self, message: &T, channel: Channel);
    pub fn receive_messages<T: DeserializeOwned>(&mut self) -> Vec<(u64, T)>;
    pub fn connected_clients(&self) -> Vec<u64>;
}
```

### Replication

Entity replication.

```rust
#[derive(Component)]
pub struct Replicated {
    pub owner: Option<u64>,   // Client ID
    pub priority: f32,         // Replication priority
}

pub struct ReplicationConfig {
    pub delta_compression: bool,
    pub interpolation: bool,
    pub extrapolation: bool,
}
```

---

## AI

### BehaviorTree

Behavior tree for AI.

```rust
pub struct BehaviorTree {
    pub root: Box<dyn BehaviorNode>,
    pub blackboard: Blackboard,
}

impl BehaviorTree {
    pub fn tick(&mut self, delta: f32) -> NodeStatus;
}

pub enum NodeStatus {
    Success,
    Failure,
    Running,
}

// Node types
pub struct Sequence { pub children: Vec<Box<dyn BehaviorNode>> }
pub struct Selector { pub children: Vec<Box<dyn BehaviorNode>> }
pub struct Parallel { pub children: Vec<Box<dyn BehaviorNode>>, pub success_count: u32 }
pub struct Repeat { pub child: Box<dyn BehaviorNode>, pub count: Option<u32> }
pub struct Invert { pub child: Box<dyn BehaviorNode> }
pub struct Cooldown { pub child: Box<dyn BehaviorNode>, pub duration: f32 }
```

### Pathfinding

A* pathfinding.

```rust
pub struct Pathfinder {
    pub navmesh: NavMesh,
}

impl Pathfinder {
    pub fn find_path(&self, start: Vec3, end: Vec3) -> Option<Vec<Vec3>>;
    pub fn smooth_path(&self, path: &[Vec3]) -> Vec<Vec3>;
}

pub struct NavMesh {
    pub triangles: Vec<Triangle>,
}
```

### SteeringBehaviors

Steering behaviors for smooth movement.

```rust
pub struct SteeringBehaviors {
    pub max_speed: f32,
    pub max_force: f32,
}

impl SteeringBehaviors {
    pub fn seek(&self, position: Vec3, target: Vec3, velocity: Vec3) -> Vec3;
    pub fn flee(&self, position: Vec3, target: Vec3, velocity: Vec3) -> Vec3;
    pub fn arrive(&self, position: Vec3, target: Vec3, velocity: Vec3, slowing_distance: f32) -> Vec3;
    pub fn wander(&self, velocity: Vec3, wander_angle: &mut f32) -> Vec3;
    pub fn pursuit(&self, position: Vec3, target_pos: Vec3, target_vel: Vec3, velocity: Vec3) -> Vec3;
    pub fn evade(&self, position: Vec3, target_pos: Vec3, target_vel: Vec3, velocity: Vec3) -> Vec3;
    pub fn separation(&self, position: Vec3, neighbors: &[Vec3], radius: f32) -> Vec3;
    pub fn alignment(&self, velocity: Vec3, neighbors: &[Vec3], radius: f32) -> Vec3;
    pub fn cohesion(&self, position: Vec3, neighbors: &[Vec3], radius: f32) -> Vec3;
}
```

---

## Animation

### SkeletalAnimation

Skeletal animation system.

```rust
pub struct SkeletalAnimation {
    pub skeleton: Skeleton,
    pub current_animation: Option<Handle<AnimationClip>>,
    pub time: f32,
    pub speed: f32,
}

pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub root: usize,
}

pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub local_transform: Transform,
}

pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<AnimationTrack>,
}
```

### AnimationBlending

Animation blending and transitions.

```rust
pub struct AnimationBlender {
    pub layers: Vec<AnimationLayer>,
}

pub struct AnimationLayer {
    pub weight: f32,
    pub blend_mode: BlendMode,
    pub animations: Vec<(Handle<AnimationClip>, f32)>,
}

pub enum BlendMode {
    Override,
    Additive,
    Masked { mask: BoneMask },
}
```

### IK (Inverse Kinematics)

```rust
pub struct IKChain {
    pub bones: Vec<usize>,
    pub target: Vec3,
    pub pole_target: Option<Vec3>,
}

impl IKChain {
    pub fn solve_fabrik(&mut self, skeleton: &mut Skeleton, iterations: u32);
    pub fn solve_two_bone(&mut self, skeleton: &mut Skeleton);
    pub fn solve_ccd(&mut self, skeleton: &mut Skeleton, iterations: u32);
}
```

---

## UI

### Widget

Base UI widget.

```rust
pub trait Widget {
    fn update(&mut self, input: &Input);
    fn render(&self, renderer: &mut UIRenderer);
    fn bounds(&self) -> Rect;
}

// Built-in widgets
pub struct Button {
    pub text: String,
    pub position: Vec2,
    pub size: Vec2,
    pub style: ButtonStyle,
    pub on_click: Option<Box<dyn Fn()>>,
}

pub struct Label {
    pub text: String,
    pub position: Vec2,
    pub font_size: f32,
    pub color: Color,
}

pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub position: Vec2,
    pub size: Vec2,
}
```

### Layout

UI layout system.

```rust
pub enum Layout {
    Stack { direction: StackDirection, spacing: f32 },
    Grid { columns: u32, rows: u32, spacing: Vec2 },
    Anchor { anchor: Anchor, offset: Vec2 },
}

pub enum StackDirection {
    Horizontal,
    Vertical,
}

pub enum Anchor {
    TopLeft, TopCenter, TopRight,
    MiddleLeft, MiddleCenter, MiddleRight,
    BottomLeft, BottomCenter, BottomRight,
}
```

---

## Assets

### AssetServer

Asset loading and management.

```rust
pub struct AssetServer;

impl AssetServer {
    pub fn load<T: Asset>(&self, path: &str) -> Handle<T>;
    pub fn get<T: Asset>(&self, handle: &Handle<T>) -> Option<&T>;
    pub fn unload<T: Asset>(&self, handle: Handle<T>);
}

pub struct Handle<T> {
    id: u64,
    _phantom: PhantomData<T>,
}
```

### Hot Reload

Asset hot-reloading.

```rust
pub struct HotReload {
    pub enabled: bool,
}

impl HotReload {
    pub fn watch<T: Asset>(&mut self, handle: Handle<T>, callback: impl Fn(&T) + 'static);
}
```

---

## Math

### Vec2, Vec3, Vec4

```rust
pub struct Vec2 { pub x: f32, pub y: f32 }
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
pub struct Vec4 { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }

// Common operations
impl Vec3 {
    pub fn length(&self) -> f32;
    pub fn normalize(&self) -> Self;
    pub fn dot(&self, other: Self) -> f32;
    pub fn cross(&self, other: Self) -> Self;
    pub fn lerp(&self, other: Self, t: f32) -> Self;
}
```

### Mat4

```rust
pub struct Mat4 { /* 4x4 matrix */ }

impl Mat4 {
    pub fn identity() -> Self;
    pub fn translation(v: Vec3) -> Self;
    pub fn rotation(axis: Vec3, angle: f32) -> Self;
    pub fn scale(v: Vec3) -> Self;
    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self;
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self;
}
```

### Quat

```rust
pub struct Quat { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }

impl Quat {
    pub fn identity() -> Self;
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self;
    pub fn from_euler(roll: f32, pitch: f32, yaw: f32) -> Self;
    pub fn slerp(&self, other: Self, t: f32) -> Self;
}
```

---

## Optimization

### RuntimeOptimizer

Automatic runtime optimizations.

```rust
pub struct RuntimeOptimizer {
    pub batching_enabled: bool,
    pub culling_enabled: bool,
    pub lod_enabled: bool,
}

impl RuntimeOptimizer {
    pub fn submit_draw(&self, mesh: Handle<Mesh>, material: Handle<Material>, transform: Mat4);
    pub fn flush_batch(&self);
}
```

### Profiler

Built-in performance profiler.

```rust
pub struct Profiler;

impl Profiler {
    pub fn begin_scope(name: &str);
    pub fn end_scope();
    pub fn frame_time() -> f32;
    pub fn fps() -> f32;
}

// Macro for easy profiling
profile_scope!("update_physics");
```

---

## See Also

- [Feature Showcase](FEATURE_SHOWCASE.md) - Overview of all features
- [Cookbook](COOKBOOK.md) - Common patterns and recipes
- [Migration Guides](UNITY_MIGRATION.md) - Migrating from Unity/Godot


