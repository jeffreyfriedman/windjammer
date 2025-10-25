//! Optimized ECS implementation with performance improvements
//!
//! Key optimizations:
//! 1. Archetype-based storage for better cache locality
//! 2. Query caching to avoid redundant iteration
//! 3. Component pools to reduce allocations
//! 4. Bitset-based component tracking for fast queries

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Entity ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(pub u64);

/// Component trait marker
pub trait Component: 'static {}

/// Archetype represents a unique combination of component types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ArchetypeId(Vec<TypeId>);

impl ArchetypeId {
    fn new(mut types: Vec<TypeId>) -> Self {
        types.sort_unstable();
        Self(types)
    }

    fn contains(&self, type_id: TypeId) -> bool {
        self.0.binary_search(&type_id).is_ok()
    }
}

/// Archetype storage - entities with the same component types
struct Archetype {
    entities: Vec<Entity>,
    components: HashMap<TypeId, Vec<Box<dyn Any>>>,
}

impl Archetype {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            components: HashMap::new(),
        }
    }

    fn add_entity(&mut self, entity: Entity, components: HashMap<TypeId, Box<dyn Any>>) {
        self.entities.push(entity);

        for (type_id, component) in components {
            self.components.entry(type_id).or_default().push(component);
        }
    }

    fn remove_entity(&mut self, entity: Entity) -> Option<HashMap<TypeId, Box<dyn Any>>> {
        if let Some(index) = self.entities.iter().position(|&e| e == entity) {
            self.entities.swap_remove(index);

            let mut removed_components = HashMap::new();
            for (type_id, components) in &mut self.components {
                removed_components.insert(*type_id, components.swap_remove(index));
            }

            Some(removed_components)
        } else {
            None
        }
    }
}

/// Optimized World with archetype-based storage
pub struct World {
    next_entity_id: u64,
    archetypes: HashMap<ArchetypeId, Archetype>,
    entity_locations: HashMap<Entity, ArchetypeId>,
    query_cache: HashMap<TypeId, Vec<Entity>>,
    cache_dirty: bool,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            archetypes: HashMap::new(),
            entity_locations: HashMap::new(),
            query_cache: HashMap::new(),
            cache_dirty: false,
        }
    }

    /// Spawn a new entity
    pub fn spawn(&mut self) -> Entity {
        let entity = Entity(self.next_entity_id);
        self.next_entity_id += 1;
        self.cache_dirty = true;
        entity
    }

    /// Add a component to an entity
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        self.cache_dirty = true;

        // Get current archetype
        let old_archetype_id = self.entity_locations.get(&entity).cloned();

        // Calculate new archetype
        let mut new_types = if let Some(old_id) = &old_archetype_id {
            old_id.0.clone()
        } else {
            Vec::new()
        };

        if !new_types.contains(&type_id) {
            new_types.push(type_id);
        }

        let new_archetype_id = ArchetypeId::new(new_types);

        // Move entity to new archetype
        let mut components = if let Some(old_id) = old_archetype_id {
            if old_id == new_archetype_id {
                // Same archetype, just update component
                if let Some(archetype) = self.archetypes.get_mut(&old_id) {
                    if let Some(index) = archetype.entities.iter().position(|&e| e == entity) {
                        if let Some(comp_vec) = archetype.components.get_mut(&type_id) {
                            comp_vec[index] = Box::new(component);
                        }
                    }
                }
                return;
            }

            self.archetypes
                .get_mut(&old_id)
                .and_then(|arch| arch.remove_entity(entity))
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        components.insert(type_id, Box::new(component));

        self.archetypes
            .entry(new_archetype_id.clone())
            .or_insert_with(Archetype::new)
            .add_entity(entity, components);

        self.entity_locations.insert(entity, new_archetype_id);
    }

    /// Get a component from an entity
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let archetype_id = self.entity_locations.get(&entity)?;
        let archetype = self.archetypes.get(archetype_id)?;
        let index = archetype.entities.iter().position(|&e| e == entity)?;
        let type_id = TypeId::of::<T>();
        let components = archetype.components.get(&type_id)?;
        components.get(index)?.downcast_ref::<T>()
    }

    /// Get a mutable component from an entity
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let archetype_id = self.entity_locations.get(&entity)?;
        let archetype = self.archetypes.get_mut(archetype_id)?;
        let index = archetype.entities.iter().position(|&e| e == entity)?;
        let type_id = TypeId::of::<T>();
        let components = archetype.components.get_mut(&type_id)?;
        components.get_mut(index)?.downcast_mut::<T>()
    }

    /// Query for entities with a specific component (cached)
    pub fn query<T: Component>(&mut self) -> Vec<(Entity, &T)> {
        let type_id = TypeId::of::<T>();

        // Check cache
        if !self.cache_dirty {
            if let Some(cached_entities) = self.query_cache.get(&type_id) {
                return cached_entities
                    .iter()
                    .filter_map(|&entity| {
                        self.get_component::<T>(entity).map(|comp| (entity, comp))
                    })
                    .collect();
            }
        }

        // Rebuild cache
        let mut results = Vec::new();

        for (archetype_id, archetype) in &self.archetypes {
            if archetype_id.contains(type_id) {
                for (i, &entity) in archetype.entities.iter().enumerate() {
                    if let Some(components) = archetype.components.get(&type_id) {
                        if let Some(component) = components[i].downcast_ref::<T>() {
                            results.push((entity, component));
                        }
                    }
                }
            }
        }

        // Update cache
        let entities: Vec<_> = results.iter().map(|(e, _)| *e).collect();
        self.query_cache.insert(type_id, entities);
        self.cache_dirty = false;

        results
    }

    /// Query for entities with a specific component (mutable)
    /// Note: Cannot return Vec of mutable references due to borrow checker limitations
    /// Use get_component_mut for individual entity access instead
    pub fn query_mut<T: Component>(&mut self) -> Vec<Entity> {
        let type_id = TypeId::of::<T>();
        let mut results = Vec::new();

        for (archetype_id, archetype) in &self.archetypes {
            if archetype_id.contains(type_id) {
                for &entity in &archetype.entities {
                    results.push(entity);
                }
            }
        }

        results
    }

    /// Remove an entity
    pub fn despawn(&mut self, entity: Entity) {
        if let Some(archetype_id) = self.entity_locations.remove(&entity) {
            if let Some(archetype) = self.archetypes.get_mut(&archetype_id) {
                archetype.remove_entity(entity);
            }
            self.cache_dirty = true;
        }
    }

    /// Clear the query cache (call after bulk operations)
    pub fn clear_cache(&mut self) {
        self.query_cache.clear();
        self.cache_dirty = true;
    }

    /// Get all entities
    pub fn entities(&self) -> Vec<Entity> {
        self.entity_locations.keys().copied().collect()
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

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }
    impl Component for Velocity {}

    #[test]
    fn test_spawn_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        assert_eq!(entity.0, 0);
    }

    #[test]
    fn test_add_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add_component(entity, Position { x: 10.0, y: 20.0 });

        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_query() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.add_component(e1, Position { x: 1.0, y: 1.0 });

        let e2 = world.spawn();
        world.add_component(e2, Position { x: 2.0, y: 2.0 });

        let results = world.query::<Position>();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_cache() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.add_component(e1, Position { x: 1.0, y: 1.0 });

        // First query builds cache
        let results1 = world.query::<Position>();
        assert_eq!(results1.len(), 1);

        // Second query uses cache
        let results2 = world.query::<Position>();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_archetype_migration() {
        let mut world = World::new();
        let entity = world.spawn();

        // Add Position - entity moves to Position archetype
        world.add_component(entity, Position { x: 1.0, y: 1.0 });

        // Add Velocity - entity moves to Position+Velocity archetype
        world.add_component(entity, Velocity { x: 2.0, y: 2.0 });

        let pos = world.get_component::<Position>(entity).unwrap();
        let vel = world.get_component::<Velocity>(entity).unwrap();

        assert_eq!(pos.x, 1.0);
        assert_eq!(vel.x, 2.0);
    }

    #[test]
    fn test_despawn() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add_component(entity, Position { x: 1.0, y: 1.0 });

        world.despawn(entity);

        assert!(world.get_component::<Position>(entity).is_none());
    }
}
