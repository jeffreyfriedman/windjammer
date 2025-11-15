/// System execution and scheduling
/// 
/// Systems are functions that operate on components.
/// They are the primary way to implement game logic in ECS.

use crate::ecs::World;

/// System trait
/// 
/// Systems process entities and components every frame.
/// They can read and write components, spawn/despawn entities, etc.
pub trait System: Send + Sync {
    /// Run the system
    fn run(&mut self, world: &mut World, delta: f32);
    
    /// Get system name for debugging
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Function system wrapper
/// 
/// Allows using closures as systems
pub struct FnSystem<F>
where
    F: FnMut(&mut World, f32) + Send + Sync,
{
    func: F,
    name: String,
}

impl<F> FnSystem<F>
where
    F: FnMut(&mut World, f32) + Send + Sync,
{
    /// Create a new function system
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            func,
            name: name.into(),
        }
    }
}

impl<F> System for FnSystem<F>
where
    F: FnMut(&mut World, f32) + Send + Sync,
{
    fn run(&mut self, world: &mut World, delta: f32) {
        (self.func)(world, delta);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// System scheduler
/// 
/// Manages system execution order and parallelization.
/// 
/// TODO: Implement parallel execution with dependency analysis
pub struct SystemScheduler {
    systems: Vec<Box<dyn System>>,
}

impl SystemScheduler {
    /// Create a new system scheduler
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }
    
    /// Add a system to the scheduler
    pub fn add_system<S: System + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }
    
    /// Add a function as a system
    pub fn add_fn_system<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: FnMut(&mut World, f32) + Send + Sync + 'static,
    {
        self.systems.push(Box::new(FnSystem::new(name, func)));
    }
    
    /// Run all systems
    pub fn run(&mut self, world: &mut World, delta: f32) {
        for system in &mut self.systems {
            system.run(world, delta);
        }
    }
    
    /// Get number of systems
    pub fn len(&self) -> usize {
        self.systems.len()
    }
    
    /// Check if scheduler is empty
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }
    
    /// Clear all systems
    pub fn clear(&mut self) {
        self.systems.clear();
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement advanced scheduling features:
// - Parallel execution with dependency analysis
// - System stages (PreUpdate, Update, PostUpdate, etc.)
// - System ordering within stages
// - System labels and dependencies
// - System sets and groups

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
    fn test_fn_system() {
        let mut world = World::new();
        let mut scheduler = SystemScheduler::new();
        
        // Spawn entity with position and velocity
        world.spawn()
            .with(Position { x: 0.0, y: 0.0, z: 0.0 })
            .with(Velocity { x: 1.0, y: 0.0, z: 0.0 })
            .build();
        
        // Add movement system
        scheduler.add_fn_system("move_system", |world, delta| {
            let entities: Vec<_> = world.entities().collect();
            for entity in entities {
                if let (Some(pos), Some(vel)) = (
                    world.get_component::<Position>(entity),
                    world.get_component::<Velocity>(entity),
                ) {
                    let new_pos = Position {
                        x: pos.x + vel.x * delta,
                        y: pos.y + vel.y * delta,
                        z: pos.z + vel.z * delta,
                    };
                    world.add_component(entity, new_pos);
                }
            }
        });
        
        // Run system
        scheduler.run(&mut world, 1.0);
        
        // Check that position was updated
        let entity = world.entities().next().unwrap();
        let pos = world.get_component::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
    }
}

