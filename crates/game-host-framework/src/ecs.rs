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

/// Entity builder for fluent API
pub struct EntityBuilder<'a> {
    world: &'a mut World,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    /// Add a component to the entity being built
    pub fn with<T: Component>(self, component: T) -> Self {
        self.world.add_component(self.entity, component);
        self
    }

    /// Finish building and return the entity
    pub fn build(self) -> Entity {
        self.entity
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            entities: Vec::new(),
            components: HashMap::new(),
        }
    }

    /// Spawn a new entity and return a builder
    pub fn spawn(&mut self) -> EntityBuilder<'_> {
        let entity = Entity::new(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(entity);
        EntityBuilder {
            world: self,
            entity,
        }
    }

    /// Spawn a new entity without builder (returns Entity directly)
    pub fn spawn_empty(&mut self) -> Entity {
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

    /// Query for entities with a specific component
    pub fn query<T: Component>(&self) -> Vec<(Entity, &T)> {
        let type_id = TypeId::of::<T>();
        let Some(components) = self.components.get(&type_id) else {
            return Vec::new();
        };

        components
            .iter()
            .filter_map(|(entity, component)| {
                let component = component.downcast_ref::<T>()?;
                Some((*entity, component))
            })
            .collect()
    }

    /// Query for entities with a specific component (mutable)
    pub fn query_mut<T: Component>(&mut self) -> Vec<(Entity, &mut T)> {
        let type_id = TypeId::of::<T>();
        let Some(components) = self.components.get_mut(&type_id) else {
            return Vec::new();
        };

        components
            .iter_mut()
            .filter_map(|(entity, component)| {
                let component = component.downcast_mut::<T>()?;
                Some((*entity, component))
            })
            .collect()
    }

    /// Query for entities with two components
    pub fn query2<T1: Component, T2: Component>(&self) -> Vec<(Entity, &T1, &T2)> {
        let mut results = Vec::new();

        for &entity in &self.entities {
            if let (Some(c1), Some(c2)) = (
                self.get_component::<T1>(entity),
                self.get_component::<T2>(entity),
            ) {
                results.push((entity, c1, c2));
            }
        }

        results
    }

    /// Query for entities with two components (mutable)
    pub fn query2_mut<T1: Component, T2: Component>(&mut self) -> Vec<(Entity, &mut T1, &mut T2)> {
        let type_id1 = TypeId::of::<T1>();
        let type_id2 = TypeId::of::<T2>();

        // Collect entities that have both components
        let entities_with_both: Vec<Entity> = self
            .entities
            .iter()
            .filter(|&&entity| {
                self.components
                    .get(&type_id1)
                    .and_then(|m| m.get(&entity))
                    .is_some()
                    && self
                        .components
                        .get(&type_id2)
                        .and_then(|m| m.get(&entity))
                        .is_some()
            })
            .copied()
            .collect();

        // Get mutable references to both component maps
        let components_ptr = &mut self.components
            as *mut HashMap<TypeId, HashMap<Entity, Box<dyn Any + Send + Sync>>>;

        let mut results = Vec::new();

        for entity in entities_with_both {
            unsafe {
                // SAFETY: We're creating two mutable references to different component types
                // This is safe because T1 and T2 have different TypeIds, so they access
                // different HashMaps within the components HashMap
                let components1 = &mut *components_ptr;
                let components2 = &mut *components_ptr;

                let c1 = components1
                    .get_mut(&type_id1)
                    .unwrap()
                    .get_mut(&entity)
                    .unwrap()
                    .downcast_mut::<T1>()
                    .unwrap();

                let c2 = components2
                    .get_mut(&type_id2)
                    .unwrap()
                    .get_mut(&entity)
                    .unwrap()
                    .downcast_mut::<T2>()
                    .unwrap();

                results.push((entity, c1, c2));
            }
        }

        results
    }

    /// Check if an entity has a specific component
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        let type_id = TypeId::of::<T>();
        self.components
            .get(&type_id)
            .and_then(|m| m.get(&entity))
            .is_some()
    }

    /// Remove a specific component from an entity
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        self.components
            .get_mut(&type_id)?
            .remove(&entity)?
            .downcast::<T>()
            .ok()
            .map(|boxed| *boxed)
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
        let entity = world.spawn_empty();
        assert_eq!(entity.0, 0);

        let entity2 = world.spawn_empty();
        assert_eq!(entity2.0, 1);
    }

    #[test]
    fn test_entity_builder() {
        let mut world = World::new();
        let entity = world
            .spawn()
            .with(Position { x: 10.0, y: 20.0 })
            .with(Velocity { x: 1.0, y: 2.0 })
            .build();

        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(pos.x, 10.0);

        let vel = world.get_component::<Velocity>(entity).unwrap();
        assert_eq!(vel.x, 1.0);
    }

    #[test]
    fn test_component_add_get() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });

        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_component_mutation() {
        let mut world = World::new();
        let entity = world.spawn_empty();

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
        let entity = world.spawn_empty();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });
        world.add_component(entity, Velocity { x: 1.0, y: 2.0 });

        assert!(world.get_component::<Position>(entity).is_some());
        assert!(world.get_component::<Velocity>(entity).is_some());
    }

    #[test]
    fn test_despawn() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });
        assert!(world.get_component::<Position>(entity).is_some());

        world.despawn(entity);
        assert!(world.get_component::<Position>(entity).is_none());
        assert_eq!(world.entities().len(), 0);
    }

    #[test]
    fn test_query_single() {
        let mut world = World::new();

        world.spawn().with(Position { x: 1.0, y: 2.0 }).build();
        world.spawn().with(Position { x: 3.0, y: 4.0 }).build();
        world.spawn().with(Velocity { x: 5.0, y: 6.0 }).build();

        let positions = world.query::<Position>();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_query_mut() {
        let mut world = World::new();

        world.spawn().with(Position { x: 1.0, y: 2.0 }).build();
        world.spawn().with(Position { x: 3.0, y: 4.0 }).build();

        for (_entity, pos) in world.query_mut::<Position>() {
            pos.x += 10.0;
        }

        let positions = world.query::<Position>();
        assert_eq!(positions[0].1.x, 11.0);
        assert_eq!(positions[1].1.x, 13.0);
    }

    #[test]
    fn test_query2() {
        let mut world = World::new();

        world
            .spawn()
            .with(Position { x: 1.0, y: 2.0 })
            .with(Velocity { x: 0.5, y: 0.5 })
            .build();

        world.spawn().with(Position { x: 3.0, y: 4.0 }).build();

        let results = world.query2::<Position, Velocity>();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.x, 1.0);
        assert_eq!(results[0].2.x, 0.5);
    }

    #[test]
    fn test_query2_mut() {
        let mut world = World::new();

        world
            .spawn()
            .with(Position { x: 0.0, y: 0.0 })
            .with(Velocity { x: 1.0, y: 2.0 })
            .build();

        for (_entity, pos, vel) in world.query2_mut::<Position, Velocity>() {
            pos.x += vel.x;
            pos.y += vel.y;
        }

        let results = world.query2::<Position, Velocity>();
        assert_eq!(results[0].1.x, 1.0);
        assert_eq!(results[0].1.y, 2.0);
    }

    #[test]
    fn test_has_component() {
        let mut world = World::new();
        let entity = world.spawn().with(Position { x: 1.0, y: 2.0 }).build();

        assert!(world.has_component::<Position>(entity));
        assert!(!world.has_component::<Velocity>(entity));
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let entity = world.spawn().with(Position { x: 1.0, y: 2.0 }).build();

        assert!(world.has_component::<Position>(entity));

        let removed = world.remove_component::<Position>(entity);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().x, 1.0);

        assert!(!world.has_component::<Position>(entity));
    }
}
