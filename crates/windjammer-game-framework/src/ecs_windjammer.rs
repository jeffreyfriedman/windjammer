//! Windjammer-friendly ECS API
//!
//! This module provides an ergonomic, idiomatic Windjammer API for the ECS system
//! while hiding Rust-specific implementation details.

use crate::ecs::{
    Component as RustComponent, Entity as RustEntity, System as RustSystem, World as RustWorld,
};

/// Entity - a unique game object identifier
///
/// In Windjammer, you don't need to worry about lifetimes or ownership.
/// Entities are simple IDs that reference game objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    inner: RustEntity,
}

impl Entity {
    pub(crate) fn from_rust(entity: RustEntity) -> Self {
        Self { inner: entity }
    }

    pub(crate) fn to_rust(self) -> RustEntity {
        self.inner
    }

    /// Get the numeric ID of this entity
    pub fn id(&self) -> u64 {
        self.inner.0
    }
}

/// World - the container for all game entities and their components
///
/// The World manages all entities and components in your game.
/// You can spawn entities, add components, and query for entities with specific components.
pub struct World {
    inner: RustWorld,
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            inner: RustWorld::new(),
        }
    }

    /// Spawn a new entity and return a builder to add components
    ///
    /// Example:
    /// ```
    /// let entity = world.spawn()
    ///     .with(Position { x: 100.0, y: 200.0 })
    ///     .with(Velocity { x: 1.0, y: 0.0 })
    ///     .build();
    /// ```
    pub fn spawn(&mut self) -> EntityBuilder<'_> {
        let rust_builder = self.inner.spawn();
        EntityBuilder {
            inner: Some(rust_builder),
            entity: None,
        }
    }

    /// Spawn an entity without any components
    pub fn spawn_empty(&mut self) -> Entity {
        Entity::from_rust(self.inner.spawn_empty())
    }

    /// Add a component to an existing entity
    pub fn add<T: RustComponent>(&mut self, entity: Entity, component: T) {
        self.inner.add_component(entity.to_rust(), component);
    }

    /// Get a component from an entity (immutable)
    pub fn get<T: RustComponent>(&self, entity: Entity) -> Option<&T> {
        self.inner.get_component(entity.to_rust())
    }

    /// Get a component from an entity (mutable)
    pub fn get_mut<T: RustComponent>(&mut self, entity: Entity) -> Option<&mut T> {
        self.inner.get_component_mut(entity.to_rust())
    }

    /// Check if an entity has a specific component
    pub fn has<T: RustComponent>(&self, entity: Entity) -> bool {
        self.inner.has_component::<T>(entity.to_rust())
    }

    /// Remove a component from an entity
    pub fn remove<T: RustComponent>(&mut self, entity: Entity) -> Option<T> {
        self.inner.remove_component(entity.to_rust())
    }

    /// Remove an entity and all its components
    pub fn despawn(&mut self, entity: Entity) {
        self.inner.despawn(entity.to_rust());
    }

    /// Query for all entities with a specific component
    ///
    /// Returns a list of (Entity, Component) pairs
    pub fn query<T: RustComponent>(&self) -> Vec<(Entity, &T)> {
        self.inner
            .query::<T>()
            .into_iter()
            .map(|(e, c)| (Entity::from_rust(e), c))
            .collect()
    }

    /// Query for all entities with a specific component (mutable)
    pub fn query_mut<T: RustComponent>(&mut self) -> Vec<(Entity, &mut T)> {
        self.inner
            .query_mut::<T>()
            .into_iter()
            .map(|(e, c)| (Entity::from_rust(e), c))
            .collect()
    }

    /// Query for entities with two specific components
    pub fn query2<T1: RustComponent, T2: RustComponent>(&self) -> Vec<(Entity, &T1, &T2)> {
        self.inner
            .query2::<T1, T2>()
            .into_iter()
            .map(|(e, c1, c2)| (Entity::from_rust(e), c1, c2))
            .collect()
    }

    /// Query for entities with two specific components (mutable)
    pub fn query2_mut<T1: RustComponent, T2: RustComponent>(
        &mut self,
    ) -> Vec<(Entity, &mut T1, &mut T2)> {
        self.inner
            .query2_mut::<T1, T2>()
            .into_iter()
            .map(|(e, c1, c2)| (Entity::from_rust(e), c1, c2))
            .collect()
    }

    /// Get all entities in the world
    pub fn entities(&self) -> Vec<Entity> {
        self.inner
            .entities()
            .iter()
            .map(|&e| Entity::from_rust(e))
            .collect()
    }

    /// Get the number of entities in the world
    pub fn entity_count(&self) -> usize {
        self.inner.entities().len()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity builder for fluent component addition
///
/// This builder allows you to chain `.with()` calls to add multiple components
/// to an entity before finalizing it with `.build()`.
pub struct EntityBuilder<'a> {
    inner: Option<crate::ecs::EntityBuilder<'a>>,
    entity: Option<Entity>,
}

impl<'a> EntityBuilder<'a> {
    /// Add a component to this entity
    pub fn with<T: RustComponent>(mut self, component: T) -> Self {
        if let Some(builder) = self.inner.take() {
            let new_builder = builder.with(component);
            self.inner = Some(new_builder);
        }
        self
    }

    /// Finish building the entity and return its ID
    pub fn build(mut self) -> Entity {
        if let Some(builder) = self.inner.take() {
            Entity::from_rust(builder.build())
        } else {
            self.entity.expect("EntityBuilder in invalid state")
        }
    }
}

/// System trait for game logic
///
/// Implement this trait to create systems that process entities.
/// Systems are run every frame and can query the world for entities with specific components.
///
/// Note: Systems must be safe to use across threads (this is handled automatically by Windjammer)
pub trait System: Send + Sync {
    /// Update this system
    ///
    /// `world` - The game world containing all entities and components
    /// `delta` - Time elapsed since last frame (in seconds)
    fn update(&mut self, world: &mut World, delta: f32);
}

// Adapter to make Windjammer Systems work with Rust Systems
#[allow(dead_code)]
struct SystemAdapter<T: System> {
    system: T,
}

impl<T: System> RustSystem for SystemAdapter<T> {
    fn update(&mut self, rust_world: &mut RustWorld, delta: f32) {
        let mut world = World {
            inner: std::mem::take(rust_world),
        };
        self.system.update(&mut world, delta);
        *rust_world = world.inner;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[test]
    fn test_windjammer_api() {
        let mut world = World::new();

        // Spawn with builder
        let entity = world
            .spawn()
            .with(Position { x: 10.0, y: 20.0 })
            .with(Velocity { x: 1.0, y: 2.0 })
            .build();

        // Get component
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 10.0);

        // Check has component
        assert!(world.has::<Position>(entity));
        assert!(world.has::<Velocity>(entity));
    }

    #[test]
    fn test_query() {
        let mut world = World::new();

        world.spawn().with(Position { x: 1.0, y: 2.0 }).build();
        world.spawn().with(Position { x: 3.0, y: 4.0 }).build();

        let positions = world.query::<Position>();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_query_mut() {
        let mut world = World::new();

        world
            .spawn()
            .with(Position { x: 0.0, y: 0.0 })
            .with(Velocity { x: 1.0, y: 2.0 })
            .build();

        // Update positions based on velocity
        for (_entity, pos, vel) in world.query2_mut::<Position, Velocity>() {
            pos.x += vel.x;
            pos.y += vel.y;
        }

        let positions = world.query::<Position>();
        assert_eq!(positions[0].1.x, 1.0);
        assert_eq!(positions[0].1.y, 2.0);
    }

    #[test]
    fn test_despawn() {
        let mut world = World::new();
        let entity = world.spawn().with(Position { x: 1.0, y: 2.0 }).build();

        assert_eq!(world.entity_count(), 1);
        world.despawn(entity);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let entity = world.spawn().with(Position { x: 1.0, y: 2.0 }).build();

        assert!(world.has::<Position>(entity));
        let removed = world.remove::<Position>(entity);
        assert!(removed.is_some());
        assert!(!world.has::<Position>(entity));
    }
}
