/// Archetype-based storage for fast iteration
/// 
/// Archetypes group entities with the same component types together.
/// This enables blazing-fast iteration over entities with specific components.
/// 
/// TODO: Full implementation in Phase 2
/// For now, we use sparse sets which are simpler and still performant.

use std::any::TypeId;
use std::collections::HashMap;
use crate::ecs::Entity;

/// Archetype ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchetypeId(pub usize);

/// Archetype: A unique combination of component types
/// 
/// All entities in an archetype have the exact same set of components.
/// This enables:
/// - Cache-friendly iteration (SOA layout)
/// - Fast queries (filter by archetype instead of entity)
/// - Efficient structural changes (move between archetypes)
#[derive(Debug)]
pub struct Archetype {
    /// Unique ID
    pub id: ArchetypeId,
    
    /// Component types in this archetype
    pub component_types: Vec<TypeId>,
    
    /// Entities in this archetype
    pub entities: Vec<Entity>,
    
    /// Component data (TypeId -> column of components)
    /// Each column is a Vec<T> stored as Box<dyn Any>
    pub components: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl Archetype {
    /// Create a new archetype
    pub fn new(id: ArchetypeId, component_types: Vec<TypeId>) -> Self {
        Self {
            id,
            component_types,
            entities: Vec::new(),
            components: HashMap::new(),
        }
    }
    
    /// Get the number of entities in this archetype
    pub fn len(&self) -> usize {
        self.entities.len()
    }
    
    /// Check if archetype is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// Archetype storage
/// 
/// Manages all archetypes and entity-to-archetype mapping.
pub struct ArchetypeStorage {
    /// All archetypes
    archetypes: Vec<Archetype>,
    
    /// Entity -> (ArchetypeId, index in archetype)
    entity_locations: HashMap<Entity, (ArchetypeId, usize)>,
    
    /// Component types -> ArchetypeId (for fast lookup)
    archetype_index: HashMap<Vec<TypeId>, ArchetypeId>,
}

impl ArchetypeStorage {
    /// Create a new archetype storage
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            entity_locations: HashMap::new(),
            archetype_index: HashMap::new(),
        }
    }
    
    /// Get or create an archetype for a set of component types
    pub fn get_or_create_archetype(&mut self, mut component_types: Vec<TypeId>) -> ArchetypeId {
        // Sort for consistent lookup
        component_types.sort();
        
        if let Some(&archetype_id) = self.archetype_index.get(&component_types) {
            archetype_id
        } else {
            let archetype_id = ArchetypeId(self.archetypes.len());
            let archetype = Archetype::new(archetype_id, component_types.clone());
            self.archetypes.push(archetype);
            self.archetype_index.insert(component_types, archetype_id);
            archetype_id
        }
    }
    
    /// Get an archetype by ID
    pub fn get_archetype(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.0)
    }
    
    /// Get a mutable archetype by ID
    pub fn get_archetype_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(id.0)
    }
    
    /// Get entity location (archetype + index)
    pub fn get_entity_location(&self, entity: Entity) -> Option<(ArchetypeId, usize)> {
        self.entity_locations.get(&entity).copied()
    }
    
    /// Iterate over all archetypes
    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }
}

impl Default for ArchetypeStorage {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement full archetype system:
// - Move entities between archetypes when components are added/removed
// - Efficient queries that iterate over matching archetypes
// - Table-based storage (SOA layout) for cache-friendly iteration
// - Edge tracking for fast archetype transitions

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_archetype_creation() {
        let mut storage = ArchetypeStorage::new();
        
        let type1 = TypeId::of::<i32>();
        let type2 = TypeId::of::<f32>();
        
        let arch1 = storage.get_or_create_archetype(vec![type1]);
        let arch2 = storage.get_or_create_archetype(vec![type1, type2]);
        let arch3 = storage.get_or_create_archetype(vec![type1]); // Should reuse arch1
        
        assert_eq!(arch1, arch3);
        assert_ne!(arch1, arch2);
    }
}

