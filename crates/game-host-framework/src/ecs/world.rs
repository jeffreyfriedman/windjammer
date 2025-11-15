/// ECS World - The central container for entities and components
/// 
/// Design:
/// - Manages entity lifecycle
/// - Stores components in sparse sets
/// - Provides query interface
/// - Executes systems

use std::any::TypeId;
use std::collections::HashMap;
use crate::ecs::{Entity, EntityAllocator, Component, ComponentStorage, SparseSet};

/// The ECS world
/// 
/// This is the main entry point for the ECS.
/// It manages entities, components, and systems.
pub struct World {
    /// Entity allocator
    entities: EntityAllocator,
    
    /// Component storage (TypeId -> SparseSet)
    components: HashMap<TypeId, Box<dyn ComponentStorage>>,
}

impl World {
    /// Create a new world
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            components: HashMap::new(),
        }
    }
    
    /// Spawn a new entity
    /// 
    /// Returns an EntityBuilder for adding components
    /// 
    /// Example:
    /// ```
    /// let entity = world.spawn()
    ///     .with(Position { x: 0.0, y: 0.0, z: 0.0 })
    ///     .with(Velocity { x: 1.0, y: 0.0, z: 0.0 })
    ///     .build();
    /// ```
    pub fn spawn(&mut self) -> EntityBuilder {
        let entity = self.entities.allocate();
        EntityBuilder {
            world: self,
            entity,
        }
    }
    
    /// Despawn an entity and remove all its components
    /// 
    /// Returns true if the entity was despawned, false if it didn't exist
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }
        
        // Remove all components
        for storage in self.components.values_mut() {
            storage.remove(entity);
        }
        
        // Deallocate entity
        self.entities.deallocate(entity)
    }
    
    /// Check if an entity is alive
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }
    
    /// Get the number of alive entities
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Add a component to an entity
    /// 
    /// Returns the old component if it existed
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) -> Option<T> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        
        self.get_or_create_storage::<T>()
            .insert(entity, component)
    }
    
    /// Remove a component from an entity
    /// 
    /// Returns the component if it existed
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        
        self.components.get_mut(&type_id)
            .and_then(|storage| {
                storage.as_any_mut()
                    .downcast_mut::<SparseSet<T>>()
                    .and_then(|sparse_set| sparse_set.remove(entity))
            })
    }
    
    /// Get a component from an entity
    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        
        self.components.get(&type_id)
            .and_then(|storage| {
                storage.as_any()
                    .downcast_ref::<SparseSet<T>>()
                    .and_then(|sparse_set| sparse_set.get(entity))
            })
    }
    
    /// Get a mutable component from an entity
    #[inline]
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        
        self.components.get_mut(&type_id)
            .and_then(|storage| {
                storage.as_any_mut()
                    .downcast_mut::<SparseSet<T>>()
                    .and_then(|sparse_set| sparse_set.get_mut(entity))
            })
    }
    
    /// Check if an entity has a component
    #[inline]
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        let type_id = TypeId::of::<T>();
        
        self.components.get(&type_id)
            .map(|storage| storage.contains(entity))
            .unwrap_or(false)
    }
    
    /// Get or create component storage for a type
    fn get_or_create_storage<T: Component>(&mut self) -> &mut SparseSet<T> {
        let type_id = TypeId::of::<T>();
        
        self.components.entry(type_id)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()))
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("Component storage type mismatch")
    }
    
    /// Get component storage for a type (read-only)
    pub fn get_storage<T: Component>(&self) -> Option<&SparseSet<T>> {
        let type_id = TypeId::of::<T>();
        
        self.components.get(&type_id)
            .and_then(|storage| storage.as_any().downcast_ref::<SparseSet<T>>())
    }
    
    /// Get component storage for a type (mutable)
    pub fn get_storage_mut<T: Component>(&mut self) -> Option<&mut SparseSet<T>> {
        let type_id = TypeId::of::<T>();
        
        self.components.get_mut(&type_id)
            .and_then(|storage| storage.as_any_mut().downcast_mut::<SparseSet<T>>())
    }
    
    /// Iterate over all entities
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.iter()
    }
    
    /// Clear all entities and components
    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.clear();
    }
    
    /// Query for entities with a specific component (immutable)
    /// 
    /// Example:
    /// ```
    /// let query = world.query::<&Position>();
    /// for (entity, pos) in query.iter() {
    ///     println!("Entity {:?} at ({}, {})", entity, pos.x, pos.y);
    /// }
    /// ```
    pub fn query<'w, T: Component>(&'w self) -> crate::ecs::Query<'w, &'w T> {
        crate::ecs::Query::new(self)
    }
    
    /// Query for entities with a specific component (mutable)
    /// 
    /// Example:
    /// ```
    /// let mut query = world.query_mut::<&mut Position>();
    /// for (entity, pos) in query.iter_mut() {
    ///     pos.x += 1.0;
    /// }
    /// ```
    pub fn query_mut<'w, T: Component>(&'w mut self) -> crate::ecs::QueryMut<'w, &'w mut T> {
        crate::ecs::QueryMut::new(self)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity builder for fluent API
/// 
/// Example:
/// ```
/// let entity = world.spawn()
///     .with(Position { x: 0.0, y: 0.0, z: 0.0 })
///     .with(Velocity { x: 1.0, y: 0.0, z: 0.0 })
///     .build();
/// ```
pub struct EntityBuilder<'w> {
    world: &'w mut World,
    entity: Entity,
}

impl<'w> EntityBuilder<'w> {
    /// Add a component to the entity
    pub fn with<T: Component>(self, component: T) -> Self {
        self.world.add_component(self.entity, component);
        self
    }
    
    /// Build and return the entity
    pub fn build(self) -> Entity {
        self.entity
    }
}

/// Automatically build the entity when dropped
impl<'w> Drop for EntityBuilder<'w> {
    fn drop(&mut self) {
        // Entity is already created, nothing to do
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
        z: f32,
    }
    
    #[test]
    fn test_world_spawn() {
        let mut world = World::new();
        
        let e1 = world.spawn().build();
        let e2 = world.spawn().build();
        
        assert_ne!(e1, e2);
        assert_eq!(world.entity_count(), 2);
    }
    
    #[test]
    fn test_world_despawn() {
        let mut world = World::new();
        
        let e1 = world.spawn().build();
        let e2 = world.spawn().build();
        
        assert!(world.despawn(e1));
        assert_eq!(world.entity_count(), 1);
        assert!(!world.is_alive(e1));
        assert!(world.is_alive(e2));
    }
    
    #[test]
    fn test_world_add_component() {
        let mut world = World::new();
        
        let entity = world.spawn().build();
        
        world.add_component(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
        
        assert!(world.has_component::<Position>(entity));
    }
    
    #[test]
    fn test_world_get_component() {
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let pos = Position { x: 1.0, y: 2.0, z: 3.0 };
        
        world.add_component(entity, pos.clone());
        
        assert_eq!(world.get_component::<Position>(entity), Some(&pos));
    }
    
    #[test]
    fn test_world_remove_component() {
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let pos = Position { x: 1.0, y: 2.0, z: 3.0 };
        
        world.add_component(entity, pos.clone());
        let removed = world.remove_component::<Position>(entity);
        
        assert_eq!(removed, Some(pos));
        assert!(!world.has_component::<Position>(entity));
    }
    
    #[test]
    fn test_entity_builder() {
        let mut world = World::new();
        
        let entity = world.spawn()
            .with(Position { x: 1.0, y: 2.0, z: 3.0 })
            .with(Velocity { x: 4.0, y: 5.0, z: 6.0 })
            .build();
        
        assert!(world.has_component::<Position>(entity));
        assert!(world.has_component::<Velocity>(entity));
    }
    
    #[test]
    fn test_world_clear() {
        let mut world = World::new();
        
        world.spawn()
            .with(Position { x: 1.0, y: 2.0, z: 3.0 })
            .build();
        
        world.spawn()
            .with(Position { x: 4.0, y: 5.0, z: 6.0 })
            .build();
        
        world.clear();
        
        assert_eq!(world.entity_count(), 0);
    }
}

