//! Entity-Component System for game entities

use std::collections::HashMap;

/// Entity ID (unique identifier for game entities)
pub type EntityId = u64;

/// Trait for game entities
pub trait GameEntity: Send + Sync {
    /// Update the entity
    fn update(&mut self, delta: f32);

    /// Get the entity ID
    fn id(&self) -> EntityId;
}

/// Entity wrapper
pub struct Entity {
    pub id: EntityId,
}

impl Entity {
    pub fn new(id: EntityId) -> Self {
        Self { id }
    }
}

/// World holds all entities and manages their lifecycle
pub struct World {
    entities: HashMap<EntityId, Box<dyn GameEntity>>,
    next_id: EntityId,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn spawn<E: GameEntity + 'static>(&mut self, entity: E) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(id, Box::new(entity));
        id
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    pub fn update_all(&mut self, delta: f32) {
        for entity in self.entities.values_mut() {
            entity.update(delta);
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
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

    struct TestEntity {
        id: EntityId,
        value: i32,
    }

    impl GameEntity for TestEntity {
        fn update(&mut self, _delta: f32) {
            self.value += 1;
        }

        fn id(&self) -> EntityId {
            self.id
        }
    }

    #[test]
    fn test_world_spawn() {
        let mut world = World::new();
        let entity = TestEntity { id: 0, value: 0 };
        let id = world.spawn(entity);
        assert_eq!(world.entity_count(), 1);
        assert_eq!(id, 0);
    }

    #[test]
    fn test_world_despawn() {
        let mut world = World::new();
        let entity = TestEntity { id: 0, value: 0 };
        let id = world.spawn(entity);
        world.despawn(id);
        assert_eq!(world.entity_count(), 0);
    }
}
