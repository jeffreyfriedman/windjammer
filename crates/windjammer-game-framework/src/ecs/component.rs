/// Component trait and utilities
/// 
/// Components are pure data that can be attached to entities.
/// They must be Send + Sync for parallel system execution.

use std::any::{Any, TypeId};

/// Component marker trait
/// 
/// All components must implement this trait.
/// Requirements:
/// - Send + Sync: For parallel system execution
/// - 'static: For type identification
/// - Sized: For storage in collections
pub trait Component: Send + Sync + 'static + Sized {}

/// Automatically implement Component for all types that meet requirements
impl<T: Send + Sync + 'static + Sized> Component for T {}

/// Type-erased component storage
pub trait ComponentStorage: Send + Sync {
    /// Get component as Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Get mutable component as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
    
    /// Get the TypeId of the component
    fn type_id(&self) -> TypeId;
    
    /// Remove component for an entity
    fn remove(&mut self, entity: crate::ecs::Entity) -> bool;
    
    /// Check if entity has this component
    fn contains(&self, entity: crate::ecs::Entity) -> bool;
    
    /// Get number of components
    fn len(&self) -> usize;
    
    /// Check if storage is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Clear all components
    fn clear(&mut self);
}

/// Component metadata
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// Type name for debugging
    pub type_name: &'static str,
    
    /// Type ID for identification
    pub type_id: TypeId,
    
    /// Size in bytes
    pub size: usize,
    
    /// Alignment in bytes
    pub align: usize,
}

impl ComponentInfo {
    /// Create component info for a type
    pub fn of<T: Component>() -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }
}

/// Component registry for metadata
pub struct ComponentRegistry {
    components: Vec<ComponentInfo>,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }
    
    /// Register a component type
    pub fn register<T: Component>(&mut self) {
        let info = ComponentInfo::of::<T>();
        if !self.components.iter().any(|c| c.type_id == info.type_id) {
            self.components.push(info);
        }
    }
    
    /// Get component info by TypeId
    pub fn get(&self, type_id: TypeId) -> Option<&ComponentInfo> {
        self.components.iter().find(|c| c.type_id == type_id)
    }
    
    /// Get all registered components
    pub fn iter(&self) -> impl Iterator<Item = &ComponentInfo> {
        self.components.iter()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
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
    fn test_component_trait() {
        // These types automatically implement Component
        let _pos: &dyn Component = &Position { x: 0.0, y: 0.0, z: 0.0 };
        let _vel: &dyn Component = &Velocity { x: 0.0, y: 0.0, z: 0.0 };
    }
    
    #[test]
    fn test_component_info() {
        let info = ComponentInfo::of::<Position>();
        
        assert_eq!(info.type_id, TypeId::of::<Position>());
        assert_eq!(info.size, std::mem::size_of::<Position>());
        assert_eq!(info.align, std::mem::align_of::<Position>());
    }
    
    #[test]
    fn test_component_registry() {
        let mut registry = ComponentRegistry::new();
        
        registry.register::<Position>();
        registry.register::<Velocity>();
        
        assert!(registry.get(TypeId::of::<Position>()).is_some());
        assert!(registry.get(TypeId::of::<Velocity>()).is_some());
        assert_eq!(registry.iter().count(), 2);
    }
}

