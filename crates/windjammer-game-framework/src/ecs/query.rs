/// Query system for efficient component iteration
/// 
/// Queries allow systems to iterate over entities with specific components.
/// They are the primary way to access component data in systems.

use crate::ecs::{World, Entity, Component};

/// Query for iterating over entities with specific components
/// 
/// This is a placeholder for now. Full query implementation will include:
/// - Multi-component queries
/// - Filters (With/Without)
/// - Optional components
/// - Changed detection
/// - Parallel iteration
pub struct Query<'w, T> {
    _world: &'w World,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T> Query<'w, T> {
    /// Create a new query (internal use)
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            _world: world,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Query for single component (read-only)
impl<'w, T: Component> Query<'w, &'w T> {
    /// Iterate over all entities with component T
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        let storage = self._world.get_storage::<T>();
        
        if let Some(storage) = storage {
            storage.iter()
        } else {
            // Return empty iterator if no storage exists
            [].iter().map(|_: &()| unreachable!())
        }
    }
}

/// Query for single component (mutable)
pub struct QueryMut<'w, T: Component> {
    world: &'w mut World,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T: Component> QueryMut<'w, T> {
    /// Create a new mutable query (internal use)
    pub(crate) fn new(world: &'w mut World) -> Self {
        Self {
            world,
            _marker: std::marker::PhantomData,
        }
    }
    
    /// Iterate over all entities with component T (mutable)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        let storage = self.world.get_storage_mut::<T>();
        
        if let Some(storage) = storage {
            storage.iter_mut()
        } else {
            // Return empty iterator if no storage exists
            [].iter_mut().map(|_: &mut ()| unreachable!())
        }
    }
}

// TODO: Implement advanced query features:
// - Multi-component queries: Query<(&Transform, &Velocity)>
// - Filters: Query<&Transform, With<Player>>
// - Without filters: Query<&Transform, Without<Enemy>>
// - Optional components: Query<(&Transform, Option<&Velocity>)>
// - Changed detection: Query<&Transform, Changed<Transform>>
// - Parallel iteration: query.par_iter()

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }
    
    #[test]
    fn test_query_iter() {
        let mut world = World::new();
        
        let e1 = world.spawn()
            .with(Position { x: 1.0, y: 2.0, z: 3.0 })
            .build();
        
        let e2 = world.spawn()
            .with(Position { x: 4.0, y: 5.0, z: 6.0 })
            .build();
        
        let query = Query::<&Position>::new(&world);
        let entities: Vec<_> = query.iter().map(|(e, _)| e).collect();
        
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
    }
}

