//! Entity-Component-System architecture
//!
//! Inspired by Bevy's ECS but simplified for Windjammer

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Entity ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(pub u64);

impl Entity {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Component trait - marker for types that can be attached to entities
pub trait Component: 'static + Send + Sync {}

// Blanket implementation for all types that meet the requirements
impl<T: 'static + Send + Sync> Component for T {}

/// System trait - processes entities with specific components
pub trait System: Send + Sync {
    fn update(&mut self, world: &mut World, delta: f32);
}

/// World - container for all entities and components
pub struct World {
    next_entity_id: u64,
    entities: Vec<Entity>,
    components: HashMap<TypeId, HashMap<Entity, Box<dyn Any + Send + Sync>>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            entities: Vec::new(),
            components: HashMap::new(),
        }
    }

    /// Spawn a new entity
    pub fn spawn(&mut self) -> Entity {
        let entity = Entity::new(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(entity);
        entity
    }

    /// Add a component to an entity
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        self.components
            .entry(type_id)
            .or_default()
            .insert(entity, Box::new(component));
    }

    /// Get a component from an entity
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.components
            .get(&type_id)?
            .get(&entity)?
            .downcast_ref::<T>()
    }

    /// Get a mutable component from an entity
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.components
            .get_mut(&type_id)?
            .get_mut(&entity)?
            .downcast_mut::<T>()
    }

    /// Remove an entity and all its components
    pub fn despawn(&mut self, entity: Entity) {
        self.entities.retain(|&e| e != entity);
        for components in self.components.values_mut() {
            components.remove(&entity);
        }
    }

    /// Get all entities
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[test]
    fn test_entity_creation() {
        let mut world = World::new();
        let entity = world.spawn();
        assert_eq!(entity.0, 0);

        let entity2 = world.spawn();
        assert_eq!(entity2.0, 1);
    }

    #[test]
    fn test_component_add_get() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });

        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_component_mutation() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });

        {
            let pos = world.get_component_mut::<Position>(entity).unwrap();
            pos.x = 30.0;
        }

        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(pos.x, 30.0);
    }

    #[test]
    fn test_multiple_components() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });
        world.add_component(entity, Velocity { x: 1.0, y: 2.0 });

        assert!(world.get_component::<Position>(entity).is_some());
        assert!(world.get_component::<Velocity>(entity).is_some());
    }

    #[test]
    fn test_despawn() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });
        assert!(world.get_component::<Position>(entity).is_some());

        world.despawn(entity);
        assert!(world.get_component::<Position>(entity).is_none());
        assert_eq!(world.entities().len(), 0);
    }
}
