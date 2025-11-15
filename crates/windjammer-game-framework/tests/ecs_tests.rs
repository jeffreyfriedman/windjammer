//! Unit tests for ECS (Entity Component System)
//!
//! Tests World, Entity, Component, Query, and System functionality.

use windjammer_game_framework::ecs::*;

// Test components
#[derive(Debug, Clone, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Debug, Clone, PartialEq)]
struct Name {
    value: String,
}

// ============================================================================
// World Tests
// ============================================================================

#[test]
fn test_world_creation() {
    let _world = World::new();
    println!("✅ World created");
}

#[test]
fn test_world_spawn_entity() {
    let mut world = World::new();
    let entity = world.spawn().build();
    assert!(world.is_alive(entity));
    println!("✅ Entity spawned: {:?}", entity);
}

#[test]
fn test_world_spawn_multiple_entities() {
    let mut world = World::new();
    let entity1 = world.spawn().build();
    let entity2 = world.spawn().build();
    let entity3 = world.spawn().build();
    
    assert!(world.is_alive(entity1));
    assert!(world.is_alive(entity2));
    assert!(world.is_alive(entity3));
    assert_ne!(entity1, entity2);
    assert_ne!(entity2, entity3);
    assert_ne!(entity1, entity3);
    
    println!("✅ Multiple entities spawned: {:?}, {:?}, {:?}", entity1, entity2, entity3);
}

#[test]
fn test_world_entity_count() {
    let mut world = World::new();
    assert_eq!(world.entity_count(), 0);
    
    let _e1 = world.spawn().build();
    assert_eq!(world.entity_count(), 1);
    
    let _e2 = world.spawn().build();
    assert_eq!(world.entity_count(), 2);
    
    let _e3 = world.spawn().build();
    assert_eq!(world.entity_count(), 3);
    
    println!("✅ Entity count: {}", world.entity_count());
}

#[test]
fn test_world_despawn_entity() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    assert!(world.is_alive(entity));
    assert_eq!(world.entity_count(), 1);
    
    world.despawn(entity);
    
    assert!(!world.is_alive(entity));
    assert_eq!(world.entity_count(), 0);
    
    println!("✅ Entity despawned");
}

// ============================================================================
// Component Tests
// ============================================================================

#[test]
fn test_add_component() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    world.add_component(entity, Position { x: 10.0, y: 20.0 });
    
    assert!(world.has_component::<Position>(entity));
    println!("✅ Component added");
}

#[test]
fn test_get_component() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    world.add_component(entity, Position { x: 10.0, y: 20.0 });
    
    let pos = world.get_component::<Position>(entity).unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);
    
    println!("✅ Component retrieved: ({}, {})", pos.x, pos.y);
}

#[test]
fn test_get_component_mut() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    world.add_component(entity, Position { x: 10.0, y: 20.0 });
    
    {
        let pos = world.get_component_mut::<Position>(entity).unwrap();
        pos.x = 30.0;
        pos.y = 40.0;
    }
    
    let pos = world.get_component::<Position>(entity).unwrap();
    assert_eq!(pos.x, 30.0);
    assert_eq!(pos.y, 40.0);
    
    println!("✅ Component modified: ({}, {})", pos.x, pos.y);
}

#[test]
fn test_remove_component() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    world.add_component(entity, Position { x: 10.0, y: 20.0 });
    assert!(world.has_component::<Position>(entity));
    
    world.remove_component::<Position>(entity);
    assert!(!world.has_component::<Position>(entity));
    
    println!("✅ Component removed");
}

#[test]
fn test_multiple_components() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    world.add_component(entity, Position { x: 10.0, y: 20.0 });
    world.add_component(entity, Velocity { x: 1.0, y: 2.0 });
    world.add_component(entity, Health { current: 100, max: 100 });
    
    assert!(world.has_component::<Position>(entity));
    assert!(world.has_component::<Velocity>(entity));
    assert!(world.has_component::<Health>(entity));
    
    println!("✅ Multiple components added");
}

// ============================================================================
// EntityBuilder Tests
// ============================================================================

#[test]
fn test_entity_builder_with_components() {
    let mut world = World::new();
    
    let entity = world.spawn()
        .with(Position { x: 10.0, y: 20.0 })
        .with(Velocity { x: 1.0, y: 2.0 })
        .build();
    
    assert!(world.has_component::<Position>(entity));
    assert!(world.has_component::<Velocity>(entity));
    
    let pos = world.get_component::<Position>(entity).unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);
    
    println!("✅ Entity built with components");
}

#[test]
fn test_entity_builder_empty() {
    let mut world = World::new();
    let entity = world.spawn().build();
    
    assert!(world.is_alive(entity));
    assert!(!world.has_component::<Position>(entity));
    
    println!("✅ Empty entity built");
}

// ============================================================================
// Query Tests
// ============================================================================

#[test]
fn test_query_single_component() {
    let mut world = World::new();
    
    // Create entities with Position
    let e1 = world.spawn().with(Position { x: 1.0, y: 2.0 }).build();
    let e2 = world.spawn().with(Position { x: 3.0, y: 4.0 }).build();
    let e3 = world.spawn().with(Position { x: 5.0, y: 6.0 }).build();
    
    // Query for Position (note: pass Position, not &Position)
    let query = world.query::<Position>();
    let results: Vec<_> = query.iter().collect();
    
    assert_eq!(results.len(), 3);
    println!("✅ Query found {} entities with Position", results.len());
}

#[test]
fn test_query_mutable() {
    let mut world = World::new();
    
    // Create entities with Position
    let _e1 = world.spawn().with(Position { x: 1.0, y: 2.0 }).build();
    let _e2 = world.spawn().with(Position { x: 3.0, y: 4.0 }).build();
    
    // Query and modify
    {
        let mut query = world.query_mut::<Position>();
        for (_entity, pos) in query.iter_mut() {
            pos.x += 10.0;
            pos.y += 10.0;
        }
    }
    
    // Verify changes
    let query = world.query::<Position>();
    for (_entity, pos) in query.iter() {
        assert!(pos.x >= 11.0);
        assert!(pos.y >= 12.0);
    }
    
    println!("✅ Query modified components");
}

#[test]
fn test_query_empty() {
    let mut world = World::new();
    
    // Create entities without Position
    let _e1 = world.spawn().with(Velocity { x: 1.0, y: 2.0 }).build();
    let _e2 = world.spawn().with(Health { current: 100, max: 100 }).build();
    
    // Query for Position (should be empty)
    let query = world.query::<Position>();
    let results: Vec<_> = query.iter().collect();
    
    assert_eq!(results.len(), 0);
    println!("✅ Empty query returned no results");
}

#[test]
fn test_query_filter() {
    let mut world = World::new();
    
    // Create entities with different components
    let _e1 = world.spawn()
        .with(Position { x: 1.0, y: 2.0 })
        .with(Velocity { x: 1.0, y: 1.0 })
        .build();
    
    let _e2 = world.spawn()
        .with(Position { x: 3.0, y: 4.0 })
        .build();
    
    let _e3 = world.spawn()
        .with(Velocity { x: 2.0, y: 2.0 })
        .build();
    
    // Query for Position only
    let query = world.query::<Position>();
    let results: Vec<_> = query.iter().collect();
    assert_eq!(results.len(), 2); // e1 and e2
    
    // Query for Velocity only
    let query = world.query::<Velocity>();
    let results: Vec<_> = query.iter().collect();
    assert_eq!(results.len(), 2); // e1 and e3
    
    println!("✅ Query filtering works");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_simple_movement_system() {
    let mut world = World::new();
    
    // Create entity with position and velocity
    let entity = world.spawn()
        .with(Position { x: 0.0, y: 0.0 })
        .with(Velocity { x: 1.0, y: 2.0 })
        .build();
    
    // Simulate movement (manual system)
    {
        let vel = world.get_component::<Velocity>(entity).unwrap().clone();
        let pos = world.get_component_mut::<Position>(entity).unwrap();
        pos.x += vel.x;
        pos.y += vel.y;
    }
    
    // Verify movement
    let pos = world.get_component::<Position>(entity).unwrap();
    assert_eq!(pos.x, 1.0);
    assert_eq!(pos.y, 2.0);
    
    println!("✅ Movement system works: ({}, {})", pos.x, pos.y);
}

#[test]
fn test_health_system() {
    let mut world = World::new();
    
    // Create entity with health
    let entity = world.spawn()
        .with(Health { current: 100, max: 100 })
        .build();
    
    // Take damage
    {
        let health = world.get_component_mut::<Health>(entity).unwrap();
        health.current -= 25;
    }
    
    // Verify damage
    let health = world.get_component::<Health>(entity).unwrap();
    assert_eq!(health.current, 75);
    assert_eq!(health.max, 100);
    
    println!("✅ Health system works: {}/{}", health.current, health.max);
}

#[test]
fn test_entity_lifecycle() {
    let mut world = World::new();
    
    // Spawn entity
    let entity = world.spawn()
        .with(Position { x: 10.0, y: 20.0 })
        .with(Velocity { x: 1.0, y: 2.0 })
        .build();
    
    assert!(world.is_alive(entity));
    assert_eq!(world.entity_count(), 1);
    
    // Modify components
    {
        let pos = world.get_component_mut::<Position>(entity).unwrap();
        pos.x = 30.0;
    }
    
    // Remove component
    world.remove_component::<Velocity>(entity);
    assert!(!world.has_component::<Velocity>(entity));
    assert!(world.has_component::<Position>(entity));
    
    // Despawn entity
    world.despawn(entity);
    assert!(!world.is_alive(entity));
    assert_eq!(world.entity_count(), 0);
    
    println!("✅ Entity lifecycle complete");
}

#[test]
fn test_many_entities() {
    let mut world = World::new();
    
    // Spawn 1000 entities
    let mut entities = Vec::new();
    for i in 0..1000 {
        let entity = world.spawn()
            .with(Position { x: i as f32, y: i as f32 * 2.0 })
            .build();
        entities.push(entity);
    }
    
    assert_eq!(world.entity_count(), 1000);
    
    // Query all positions
    let query = world.query::<Position>();
    let results: Vec<_> = query.iter().collect();
    assert_eq!(results.len(), 1000);
    
    // Despawn all
    for entity in entities {
        world.despawn(entity);
    }
    
    assert_eq!(world.entity_count(), 0);
    
    println!("✅ Handled 1000 entities");
}

#[test]
fn test_component_independence() {
    let mut world = World::new();
    
    let e1 = world.spawn().with(Position { x: 1.0, y: 1.0 }).build();
    let e2 = world.spawn().with(Position { x: 2.0, y: 2.0 }).build();
    
    // Modify e1
    {
        let pos = world.get_component_mut::<Position>(e1).unwrap();
        pos.x = 100.0;
    }
    
    // Verify e2 is unchanged
    let pos2 = world.get_component::<Position>(e2).unwrap();
    assert_eq!(pos2.x, 2.0);
    assert_eq!(pos2.y, 2.0);
    
    println!("✅ Components are independent");
}

#[test]
fn test_string_component() {
    let mut world = World::new();
    
    let entity = world.spawn()
        .with(Name { value: "Player".to_string() })
        .build();
    
    let name = world.get_component::<Name>(entity).unwrap();
    assert_eq!(name.value, "Player");
    
    println!("✅ String component works: {}", name.value);
}

