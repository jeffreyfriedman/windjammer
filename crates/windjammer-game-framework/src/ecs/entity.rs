/// Entity: Lightweight identifier for game objects
/// 
/// Design:
/// - 64-bit ID (32-bit index + 32-bit generation)
/// - Generation for detecting use-after-free
/// - Copy + Clone for ergonomics
/// - Hash + Eq for use in collections

use std::fmt;

/// Entity identifier
/// 
/// Entities are just IDs - all data lives in components.
/// The generation counter prevents use-after-free bugs.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    /// Create a new entity (internal use only)
    #[inline]
    pub(crate) fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    
    /// Get the entity index
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
    
    /// Get the entity generation
    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }
    
    /// Pack into u64 for compact storage
    #[inline]
    pub fn to_bits(&self) -> u64 {
        ((self.generation as u64) << 32) | (self.index as u64)
    }
    
    /// Unpack from u64
    #[inline]
    pub fn from_bits(bits: u64) -> Self {
        Self {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        }
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({}v{})", self.index, self.generation)
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

/// Entity allocator with generation tracking
/// 
/// Design:
/// - Free list for reusing entity slots
/// - Generation counter prevents use-after-free
/// - O(1) allocation and deallocation
pub struct EntityAllocator {
    /// All entity generations (index -> generation)
    generations: Vec<u32>,
    
    /// Free list of available entity indices
    free_list: Vec<u32>,
    
    /// Number of alive entities
    alive_count: usize,
}

impl EntityAllocator {
    /// Create a new entity allocator
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_list: Vec::new(),
            alive_count: 0,
        }
    }
    
    /// Allocate a new entity
    /// 
    /// Performance: O(1)
    pub fn allocate(&mut self) -> Entity {
        if let Some(index) = self.free_list.pop() {
            // Reuse a freed entity slot
            let generation = self.generations[index as usize];
            self.alive_count += 1;
            Entity::new(index, generation)
        } else {
            // Allocate a new entity slot
            let index = self.generations.len() as u32;
            let generation = 0;
            self.generations.push(generation);
            self.alive_count += 1;
            Entity::new(index, generation)
        }
    }
    
    /// Deallocate an entity
    /// 
    /// Performance: O(1)
    pub fn deallocate(&mut self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        
        // Check if entity exists
        if index >= self.generations.len() {
            return false;
        }
        
        // Check if generation matches (prevents use-after-free)
        if self.generations[index] != entity.generation() {
            return false;
        }
        
        // Increment generation and add to free list
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_list.push(entity.index());
        self.alive_count -= 1;
        
        true
    }
    
    /// Check if an entity is alive
    /// 
    /// Performance: O(1)
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        index < self.generations.len() 
            && self.generations[index] == entity.generation()
    }
    
    /// Get the number of alive entities
    #[inline]
    pub fn len(&self) -> usize {
        self.alive_count
    }
    
    /// Check if allocator is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.alive_count == 0
    }
    
    /// Get total capacity (including freed slots)
    #[inline]
    pub fn capacity(&self) -> usize {
        self.generations.len()
    }
    
    /// Iterate over all alive entities
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.generations
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.free_list.contains(&(*index as u32)))
            .map(|(index, &generation)| Entity::new(index as u32, generation))
    }
    
    /// Clear all entities
    pub fn clear(&mut self) {
        self.generations.clear();
        self.free_list.clear();
        self.alive_count = 0;
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_allocate() {
        let mut allocator = EntityAllocator::new();
        
        let e1 = allocator.allocate();
        let e2 = allocator.allocate();
        let e3 = allocator.allocate();
        
        assert_eq!(e1.index(), 0);
        assert_eq!(e2.index(), 1);
        assert_eq!(e3.index(), 2);
        assert_eq!(allocator.len(), 3);
    }
    
    #[test]
    fn test_entity_deallocate() {
        let mut allocator = EntityAllocator::new();
        
        let e1 = allocator.allocate();
        let e2 = allocator.allocate();
        
        assert!(allocator.deallocate(e1));
        assert_eq!(allocator.len(), 1);
        assert!(!allocator.is_alive(e1));
        assert!(allocator.is_alive(e2));
    }
    
    #[test]
    fn test_entity_reuse() {
        let mut allocator = EntityAllocator::new();
        
        let e1 = allocator.allocate();
        allocator.deallocate(e1);
        
        let e2 = allocator.allocate();
        
        // Should reuse the same index
        assert_eq!(e1.index(), e2.index());
        // But with different generation
        assert_ne!(e1.generation(), e2.generation());
    }
    
    #[test]
    fn test_entity_use_after_free() {
        let mut allocator = EntityAllocator::new();
        
        let e1 = allocator.allocate();
        allocator.deallocate(e1);
        
        // Should not be alive anymore
        assert!(!allocator.is_alive(e1));
        
        // Trying to deallocate again should fail
        assert!(!allocator.deallocate(e1));
    }
    
    #[test]
    fn test_entity_to_bits() {
        let entity = Entity::new(42, 7);
        let bits = entity.to_bits();
        let restored = Entity::from_bits(bits);
        
        assert_eq!(entity, restored);
    }
}

