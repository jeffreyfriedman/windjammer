// windjammer-runtime::game - Game engine runtime backing std::game

use std::any::{Any, TypeId};
use std::collections::HashMap;

// Sub-modules
pub mod input;
pub mod render;

// Re-export key types
pub use input::{Input, Key, MouseButton};
pub use render::{Camera2D, RenderBackend, Renderer2D, SpriteBatch};

/// Entity ID - unique identifier for game entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// Component trait - marker for game components
pub trait Component: Any + Send + Sync {}

// Blanket impl for any type that is Any + Send + Sync
// This allows user-defined structs to automatically be components
impl<T: Any + Send + Sync> Component for T {}

/// World - ECS world containing all entities and components
pub struct World {
    next_entity_id: u64,
    entities: Vec<EntityId>,
    components: HashMap<TypeId, HashMap<EntityId, Box<dyn Any + Send + Sync>>>,
}

impl World {
    pub fn new() -> Self {
        World {
            next_entity_id: 0,
            entities: Vec::new(),
            components: HashMap::new(),
        }
    }

    /// Create a new entity
    pub fn create_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(id);
        id
    }

    /// Add a component to an entity
    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) {
        let type_id = TypeId::of::<T>();
        self.components
            .entry(type_id)
            .or_insert_with(HashMap::new)
            .insert(entity, Box::new(component));
    }

    /// Get a component from an entity
    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.components
            .get(&type_id)?
            .get(&entity)?
            .downcast_ref::<T>()
    }

    /// Get a mutable component from an entity
    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.components
            .get_mut(&type_id)?
            .get_mut(&entity)?
            .downcast_mut::<T>()
    }

    /// Remove a component from an entity
    pub fn remove_component<T: Component>(&mut self, entity: EntityId) -> bool {
        let type_id = TypeId::of::<T>();
        if let Some(components) = self.components.get_mut(&type_id) {
            components.remove(&entity).is_some()
        } else {
            false
        }
    }

    /// Delete an entity and all its components
    pub fn delete_entity(&mut self, entity: EntityId) {
        self.entities.retain(|&e| e != entity);
        for components in self.components.values_mut() {
            components.remove(&entity);
        }
    }

    /// Get all entities
    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    /// Query entities with a specific component
    pub fn query<T: Component>(&self) -> Vec<(EntityId, &T)> {
        let type_id = TypeId::of::<T>();
        if let Some(components) = self.components.get(&type_id) {
            components
                .iter()
                .filter_map(|(entity, component)| {
                    component.downcast_ref::<T>().map(|c| (*entity, c))
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Math Types - 2D and 3D vectors and matrices
// ============================================================================

/// 2D Vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    pub fn zero() -> Self {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Vec2 {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            *self
        }
    }

    pub fn dot(&self, other: &Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }
}

/// 3D Vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn zero() -> Self {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Vec3 {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }

    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vec3) -> Self {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

/// 4x4 Matrix for transformations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub data: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        Mat4 {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Mat4 {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x, y, z, 1.0],
            ],
        }
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Mat4 {
            data: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_z(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Mat4 {
            data: [
                [cos, sin, 0.0, 0.0],
                [-sin, cos, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

// ============================================================================
// Common Game Components
// ============================================================================

/// Transform component - position, rotation, scale
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    pub fn new(position: Vec3) -> Self {
        Transform {
            position,
            rotation: Vec3::zero(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn identity() -> Self {
        Transform {
            position: Vec3::zero(),
            rotation: Vec3::zero(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

/// Velocity component - movement speed
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

impl Velocity {
    pub fn new(linear: Vec3) -> Self {
        Velocity {
            linear,
            angular: Vec3::zero(),
        }
    }

    pub fn zero() -> Self {
        Velocity {
            linear: Vec3::zero(),
            angular: Vec3::zero(),
        }
    }
}

/// Sprite component - 2D rendering
#[derive(Debug, Clone)]
pub struct Sprite {
    pub texture_path: String,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4], // RGBA
}

impl Sprite {
    pub fn new(texture_path: &str, width: f32, height: f32) -> Self {
        Sprite {
            texture_path: texture_path.to_string(),
            width,
            height,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }
}

/// Mesh component - 3D rendering
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
}

impl Mesh {
    pub fn cube(size: f32) -> Self {
        let half = size / 2.0;
        Mesh {
            vertices: vec![
                Vec3::new(-half, -half, -half),
                Vec3::new(half, -half, -half),
                Vec3::new(half, half, -half),
                Vec3::new(-half, half, -half),
                Vec3::new(-half, -half, half),
                Vec3::new(half, -half, half),
                Vec3::new(half, half, half),
                Vec3::new(-half, half, half),
            ],
            indices: vec![
                0, 1, 2, 2, 3, 0, // front
                1, 5, 6, 6, 2, 1, // right
                5, 4, 7, 7, 6, 5, // back
                4, 0, 3, 3, 7, 4, // left
                3, 2, 6, 6, 7, 3, // top
                4, 5, 1, 1, 0, 4, // bottom
            ],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Player marker component
#[derive(Debug, Clone, Copy)]
pub struct Player;

impl Player {
    pub fn new() -> Self {
        Player
    }
}

/// Goal marker component
#[derive(Debug, Clone, Copy)]
pub struct Goal;

impl Goal {
    pub fn new() -> Self {
        Goal
    }
}

/// Collider component for physics
#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub width: f32,
    pub height: f32,
}

impl Collider {
    pub fn new(width: f32, height: f32) -> Self {
        Collider { width, height }
    }
}

/// Color helper
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub fn to_f32_array(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

// Update Sprite to accept Color
impl Sprite {
    pub fn new_with_color(texture_path: &str, width: f32, height: f32, color: Color) -> Self {
        Sprite {
            texture_path: texture_path.to_string(),
            width,
            height,
            color: color.to_f32_array(),
        }
    }
}

// ============================================================================
// Game Loop and Systems
// ============================================================================

/// System trait - processes entities each frame
pub trait System {
    fn update(&mut self, world: &mut World, delta_time: f32);
}

/// Game application
pub struct Game {
    world: World,
    systems: Vec<Box<dyn System>>,
}

impl Game {
    pub fn new() -> Self {
        Game {
            world: World::new(),
            systems: Vec::new(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
    }

    pub fn update(&mut self, delta_time: f32) {
        for system in &mut self.systems {
            system.update(&mut self.world, delta_time);
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper functions for std::game API
// ============================================================================

pub fn create_entity(world: &mut World) -> EntityId {
    world.create_entity()
}

pub fn add_transform(world: &mut World, entity: EntityId, position: Vec3) {
    world.add_component(entity, Transform::new(position));
}

pub fn add_velocity(world: &mut World, entity: EntityId, velocity: Vec3) {
    world.add_component(entity, Velocity::new(velocity));
}

pub fn add_sprite(world: &mut World, entity: EntityId, texture: &str, width: f32, height: f32) {
    world.add_component(entity, Sprite::new(texture, width, height));
}

pub fn add_mesh(world: &mut World, entity: EntityId, mesh: Mesh) {
    world.add_component(entity, mesh);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_create_entity() {
        let mut world = World::new();
        let entity1 = world.create_entity();
        let entity2 = world.create_entity();
        assert_ne!(entity1, entity2);
        assert_eq!(world.entities().len(), 2);
    }

    #[test]
    fn test_add_and_get_component() {
        let mut world = World::new();
        let entity = world.create_entity();
        let transform = Transform::new(Vec3::new(1.0, 2.0, 3.0));
        world.add_component(entity, transform);

        let retrieved = world.get_component::<Transform>(entity);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().position.x, 1.0);
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Transform::identity());
        assert!(world.get_component::<Transform>(entity).is_some());

        world.remove_component::<Transform>(entity);
        assert!(world.get_component::<Transform>(entity).is_none());
    }

    #[test]
    fn test_delete_entity() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Transform::identity());
        assert_eq!(world.entities().len(), 1);

        world.delete_entity(entity);
        assert_eq!(world.entities().len(), 0);
        assert!(world.get_component::<Transform>(entity).is_none());
    }

    #[test]
    fn test_query() {
        let mut world = World::new();
        let entity1 = world.create_entity();
        let entity2 = world.create_entity();
        world.add_component(entity1, Transform::new(Vec3::new(1.0, 0.0, 0.0)));
        world.add_component(entity2, Transform::new(Vec3::new(2.0, 0.0, 0.0)));

        let results = world.query::<Transform>();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_vec2_operations() {
        let v1 = Vec2::new(3.0, 4.0);
        assert_eq!(v1.length(), 5.0);

        let v2 = v1.normalize();
        assert!((v2.length() - 1.0).abs() < 0.0001);

        let v3 = Vec2::new(1.0, 0.0);
        let v4 = Vec2::new(0.0, 1.0);
        assert_eq!(v3.dot(&v4), 0.0);
    }

    #[test]
    fn test_vec3_operations() {
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let cross = v1.cross(&v2);
        assert_eq!(cross.x, 0.0);
        assert_eq!(cross.y, 0.0);
        assert_eq!(cross.z, 1.0);
    }

    #[test]
    fn test_mat4_identity() {
        let m = Mat4::identity();
        assert_eq!(m.data[0][0], 1.0);
        assert_eq!(m.data[1][1], 1.0);
        assert_eq!(m.data[2][2], 1.0);
        assert_eq!(m.data[3][3], 1.0);
    }

    #[test]
    fn test_mesh_cube() {
        let cube = Mesh::cube(2.0);
        assert_eq!(cube.vertices.len(), 8);
        assert_eq!(cube.indices.len(), 36);
    }
}
