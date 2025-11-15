/// Component storage implementations
/// 
/// Sparse Set: Fast add/remove, cache-friendly iteration
/// Performance: O(1) for all operations

use std::any::{Any, TypeId};
use crate::ecs::{Entity, Component, ComponentStorage};

/// Sparse set component storage
/// 
/// Design:
/// - Sparse array: entity index -> dense index (O(1) lookup)
/// - Dense arrays: packed entities and components (cache-friendly iteration)
/// 
/// Performance:
/// - Insert: O(1)
/// - Remove: O(1) (swap-remove)
/// - Get: O(1)
/// - Iteration: Cache-friendly (linear scan of dense array)
pub struct SparseSet<T: Component> {
    /// Sparse array: entity index -> dense index
    /// None means entity doesn't have this component
    sparse: Vec<Option<usize>>,
    
    /// Dense array of entities (packed)
    dense_entities: Vec<Entity>,
    
    /// Dense array of components (packed, same order as dense_entities)
    dense_components: Vec<T>,
}

impl<T: Component> SparseSet<T> {
    /// Create a new sparse set
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense_entities: Vec::new(),
            dense_components: Vec::new(),
        }
    }
    
    /// Insert a component for an entity
    /// 
    /// Returns the old component if it existed
    /// 
    /// Performance: O(1)
    pub fn insert(&mut self, entity: Entity, component: T) -> Option<T> {
        let index = entity.index() as usize;
        
        // Ensure sparse array is large enough
        if index >= self.sparse.len() {
            self.sparse.resize(index + 1, None);
        }
        
        if let Some(dense_index) = self.sparse[index] {
            // Entity already has this component, replace it
            Some(std::mem::replace(&mut self.dense_components[dense_index], component))
        } else {
            // Entity doesn't have this component, add it
            let dense_index = self.dense_entities.len();
            self.sparse[index] = Some(dense_index);
            self.dense_entities.push(entity);
            self.dense_components.push(component);
            None
        }
    }
    
    /// Remove a component for an entity
    /// 
    /// Returns the component if it existed
    /// 
    /// Performance: O(1) (swap-remove)
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let index = entity.index() as usize;
        
        if index >= self.sparse.len() {
            return None;
        }
        
        if let Some(dense_index) = self.sparse[index] {
            // Remove from dense arrays using swap-remove
            let component = self.dense_components.swap_remove(dense_index);
            let removed_entity = self.dense_entities.swap_remove(dense_index);
            
            // Update sparse array for the swapped entity
            if dense_index < self.dense_entities.len() {
                let swapped_entity = self.dense_entities[dense_index];
                self.sparse[swapped_entity.index() as usize] = Some(dense_index);
            }
            
            // Mark entity as not having this component
            self.sparse[index] = None;
            
            debug_assert_eq!(removed_entity, entity);
            Some(component)
        } else {
            None
        }
    }
    
    /// Get a component for an entity
    /// 
    /// Performance: O(1)
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let index = entity.index() as usize;
        
        if index >= self.sparse.len() {
            return None;
        }
        
        self.sparse[index].map(|dense_index| &self.dense_components[dense_index])
    }
    
    /// Get a mutable component for an entity
    /// 
    /// Performance: O(1)
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let index = entity.index() as usize;
        
        if index >= self.sparse.len() {
            return None;
        }
        
        self.sparse[index].map(|dense_index| &mut self.dense_components[dense_index])
    }
    
    /// Check if an entity has this component
    /// 
    /// Performance: O(1)
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        index < self.sparse.len() && self.sparse[index].is_some()
    }
    
    /// Get the number of components
    #[inline]
    pub fn len(&self) -> usize {
        self.dense_entities.len()
    }
    
    /// Check if storage is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense_entities.is_empty()
    }
    
    /// Clear all components
    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense_entities.clear();
        self.dense_components.clear();
    }
    
    /// Iterate over all (entity, component) pairs
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense_entities.iter().zip(self.dense_components.iter())
            .map(|(&entity, component)| (entity, component))
    }
    
    /// Iterate over all (entity, component) pairs mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.dense_entities.iter().zip(self.dense_components.iter_mut())
            .map(|(&entity, component)| (entity, component))
    }
    
    /// Get entities slice (for efficient iteration)
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.dense_entities
    }
    
    /// Get components slice (for efficient iteration)
    #[inline]
    pub fn components(&self) -> &[T] {
        &self.dense_components
    }
    
    /// Get mutable components slice (for efficient iteration)
    #[inline]
    pub fn components_mut(&mut self) -> &mut [T] {
        &mut self.dense_components
    }
}

impl<T: Component> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement ComponentStorage trait for type erasure
impl<T: Component> ComponentStorage for SparseSet<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn remove(&mut self, entity: Entity) -> bool {
        SparseSet::remove(self, entity).is_some()
    }
    
    fn contains(&self, entity: Entity) -> bool {
        SparseSet::contains(self, entity)
    }
    
    fn len(&self) -> usize {
        SparseSet::len(self)
    }
    
    fn clear(&mut self) {
        SparseSet::clear(self)
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
    
    fn make_entity(index: u32) -> Entity {
        Entity::new(index, 0)
    }
    
    #[test]
    fn test_sparse_set_insert() {
        let mut storage = SparseSet::<Position>::new();
        
        let e1 = make_entity(0);
        let e2 = make_entity(1);
        
        storage.insert(e1, Position { x: 1.0, y: 2.0, z: 3.0 });
        storage.insert(e2, Position { x: 4.0, y: 5.0, z: 6.0 });
        
        assert_eq!(storage.len(), 2);
        assert!(storage.contains(e1));
        assert!(storage.contains(e2));
    }
    
    #[test]
    fn test_sparse_set_get() {
        let mut storage = SparseSet::<Position>::new();
        
        let e1 = make_entity(0);
        let pos = Position { x: 1.0, y: 2.0, z: 3.0 };
        
        storage.insert(e1, pos.clone());
        
        assert_eq!(storage.get(e1), Some(&pos));
    }
    
    #[test]
    fn test_sparse_set_remove() {
        let mut storage = SparseSet::<Position>::new();
        
        let e1 = make_entity(0);
        let e2 = make_entity(1);
        
        storage.insert(e1, Position { x: 1.0, y: 2.0, z: 3.0 });
        storage.insert(e2, Position { x: 4.0, y: 5.0, z: 6.0 });
        
        let removed = storage.remove(e1);
        
        assert!(removed.is_some());
        assert_eq!(storage.len(), 1);
        assert!(!storage.contains(e1));
        assert!(storage.contains(e2));
    }
    
    #[test]
    fn test_sparse_set_replace() {
        let mut storage = SparseSet::<Position>::new();
        
        let e1 = make_entity(0);
        let pos1 = Position { x: 1.0, y: 2.0, z: 3.0 };
        let pos2 = Position { x: 4.0, y: 5.0, z: 6.0 };
        
        storage.insert(e1, pos1.clone());
        let old = storage.insert(e1, pos2.clone());
        
        assert_eq!(old, Some(pos1));
        assert_eq!(storage.get(e1), Some(&pos2));
        assert_eq!(storage.len(), 1);
    }
    
    #[test]
    fn test_sparse_set_iteration() {
        let mut storage = SparseSet::<Position>::new();
        
        let e1 = make_entity(0);
        let e2 = make_entity(1);
        let e3 = make_entity(2);
        
        storage.insert(e1, Position { x: 1.0, y: 2.0, z: 3.0 });
        storage.insert(e2, Position { x: 4.0, y: 5.0, z: 6.0 });
        storage.insert(e3, Position { x: 7.0, y: 8.0, z: 9.0 });
        
        let entities: Vec<_> = storage.iter().map(|(e, _)| e).collect();
        
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(entities.contains(&e3));
    }
    
    #[test]
    fn test_sparse_set_clear() {
        let mut storage = SparseSet::<Position>::new();
        
        let e1 = make_entity(0);
        let e2 = make_entity(1);
        
        storage.insert(e1, Position { x: 1.0, y: 2.0, z: 3.0 });
        storage.insert(e2, Position { x: 4.0, y: 5.0, z: 6.0 });
        
        storage.clear();
        
        assert_eq!(storage.len(), 0);
        assert!(!storage.contains(e1));
        assert!(!storage.contains(e2));
    }
}

