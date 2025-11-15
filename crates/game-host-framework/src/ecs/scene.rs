/// Scene graph with transform hierarchy
/// 
/// Manages parent-child relationships and transform propagation.
/// Inspired by Unity and Godot's scene systems.

use crate::ecs::{Entity, World, Component};
use crate::math::{Vec3, Quat, Mat4};

/// Transform component
/// 
/// Represents position, rotation, and scale in 3D space.
/// Supports both local and world transforms with hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    /// Local position relative to parent
    pub position: Vec3,
    
    /// Local rotation relative to parent
    pub rotation: Quat,
    
    /// Local scale relative to parent
    pub scale: Vec3,
    
    /// Cached world transform matrix
    world_matrix: Mat4,
    
    /// Dirty flag for transform updates
    dirty: bool,
}

impl Transform {
    /// Create a new transform at the origin
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            world_matrix: Mat4::IDENTITY,
            dirty: true,
        }
    }
    
    /// Create a transform at a specific position
    pub fn at(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            world_matrix: Mat4::IDENTITY,
            dirty: true,
        }
    }
    
    /// Get the local transform matrix
    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
    
    /// Get the world transform matrix
    pub fn world_matrix(&self) -> Mat4 {
        self.world_matrix
    }
    
    /// Mark transform as dirty (needs update)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Check if transform is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Update world matrix from parent
    pub fn update_world_matrix(&mut self, parent_matrix: Option<Mat4>) {
        let local = self.local_matrix();
        
        self.world_matrix = if let Some(parent) = parent_matrix {
            parent * local
        } else {
            local
        };
        
        self.dirty = false;
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

/// Parent component
/// 
/// Marks an entity as having a parent in the scene graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub Entity);

/// Children component
/// 
/// Stores the children of an entity in the scene graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Children(pub Vec<Entity>);

impl Children {
    /// Create an empty children list
    pub fn new() -> Self {
        Self(Vec::new())
    }
    
    /// Add a child
    pub fn add(&mut self, child: Entity) {
        if !self.0.contains(&child) {
            self.0.push(child);
        }
    }
    
    /// Remove a child
    pub fn remove(&mut self, child: Entity) {
        self.0.retain(|&e| e != child);
    }
    
    /// Get children slice
    pub fn as_slice(&self) -> &[Entity] {
        &self.0
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene graph system
/// 
/// Manages parent-child relationships and transform propagation.
pub struct SceneGraph;

impl SceneGraph {
    /// Set parent-child relationship
    /// 
    /// This will:
    /// 1. Add Parent component to child
    /// 2. Add child to parent's Children component
    /// 3. Mark transforms as dirty
    pub fn set_parent(world: &mut World, child: Entity, parent: Entity) {
        // Add Parent component to child
        world.add_component(child, Parent(parent));
        
        // Add child to parent's Children
        if let Some(children) = world.get_component_mut::<Children>(parent) {
            children.add(child);
        } else {
            let mut children = Children::new();
            children.add(child);
            world.add_component(parent, children);
        }
        
        // Mark transforms as dirty
        if let Some(transform) = world.get_component_mut::<Transform>(child) {
            transform.mark_dirty();
        }
    }
    
    /// Remove parent-child relationship
    pub fn remove_parent(world: &mut World, child: Entity) {
        // Get parent entity
        let parent = if let Some(Parent(parent)) = world.get_component::<Parent>(child) {
            *parent
        } else {
            return; // No parent
        };
        
        // Remove child from parent's Children
        if let Some(children) = world.get_component_mut::<Children>(parent) {
            children.remove(child);
        }
        
        // Remove Parent component from child
        world.remove_component::<Parent>(child);
        
        // Mark transform as dirty
        if let Some(transform) = world.get_component_mut::<Transform>(child) {
            transform.mark_dirty();
        }
    }
    
    /// Update all transforms in the scene graph
    /// 
    /// This propagates transforms from parents to children recursively.
    pub fn update_transforms(world: &mut World) {
        // Find all root entities (no parent)
        let roots: Vec<Entity> = world.entities()
            .filter(|&e| !world.has_component::<Parent>(e) && world.has_component::<Transform>(e))
            .collect();
        
        // Update each root and its children recursively
        for root in roots {
            Self::update_transform_recursive(world, root, None);
        }
    }
    
    /// Update transform recursively
    fn update_transform_recursive(world: &mut World, entity: Entity, parent_matrix: Option<Mat4>) {
        // Update this entity's transform
        if let Some(transform) = world.get_component_mut::<Transform>(entity) {
            if transform.is_dirty() || parent_matrix.is_some() {
                transform.update_world_matrix(parent_matrix);
            }
        }
        
        // Get world matrix for children
        let world_matrix = world.get_component::<Transform>(entity)
            .map(|t| t.world_matrix());
        
        // Collect children to avoid borrow checker issues
        let children_vec = world.get_component::<Children>(entity)
            .map(|c| c.as_slice().to_vec())
            .unwrap_or_default();
        
        // Update children
        for child in children_vec {
            Self::update_transform_recursive(world, child, world_matrix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transform_creation() {
        let transform = Transform::new();
        assert_eq!(transform.position, Vec3::ZERO);
        assert_eq!(transform.rotation, Quat::IDENTITY);
        assert_eq!(transform.scale, Vec3::ONE);
    }
    
    #[test]
    fn test_transform_at() {
        let transform = Transform::at(1.0, 2.0, 3.0);
        assert_eq!(transform.position, Vec3::new(1.0, 2.0, 3.0));
    }
    
    #[test]
    fn test_parent_child() {
        let mut world = World::new();
        
        let parent = world.spawn()
            .with(Transform::new())
            .build();
        
        let child = world.spawn()
            .with(Transform::new())
            .build();
        
        SceneGraph::set_parent(&mut world, child, parent);
        
        // Check parent component
        assert!(world.has_component::<Parent>(child));
        assert_eq!(world.get_component::<Parent>(child), Some(&Parent(parent)));
        
        // Check children component
        assert!(world.has_component::<Children>(parent));
        let children = world.get_component::<Children>(parent).unwrap();
        assert_eq!(children.as_slice(), &[child]);
    }
    
    #[test]
    fn test_remove_parent() {
        let mut world = World::new();
        
        let parent = world.spawn()
            .with(Transform::new())
            .build();
        
        let child = world.spawn()
            .with(Transform::new())
            .build();
        
        SceneGraph::set_parent(&mut world, child, parent);
        SceneGraph::remove_parent(&mut world, child);
        
        // Check parent component removed
        assert!(!world.has_component::<Parent>(child));
        
        // Check child removed from parent's children
        let children = world.get_component::<Children>(parent).unwrap();
        assert_eq!(children.as_slice(), &[]);
    }
    
    #[test]
    fn test_transform_hierarchy() {
        let mut world = World::new();
        
        // Create parent at (10, 0, 0)
        let parent = world.spawn()
            .with(Transform::at(10.0, 0.0, 0.0))
            .build();
        
        // Create child at (5, 0, 0) local
        let child = world.spawn()
            .with(Transform::at(5.0, 0.0, 0.0))
            .build();
        
        SceneGraph::set_parent(&mut world, child, parent);
        SceneGraph::update_transforms(&mut world);
        
        // Child's world position should be (15, 0, 0)
        let child_transform = world.get_component::<Transform>(child).unwrap();
        let world_pos = child_transform.world_matrix().translation();
        
        assert!((world_pos.x - 15.0).abs() < 0.001);
        assert!((world_pos.y - 0.0).abs() < 0.001);
        assert!((world_pos.z - 0.0).abs() < 0.001);
    }
}

