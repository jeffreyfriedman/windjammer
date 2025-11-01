# Simple Platformer Demo

This document describes how to build a simple platformer using the Windjammer Game Framework.

## Features

- Player character with physics
- Platform collision detection
- Jumping mechanics
- Simple level design

## Implementation Guide

### 1. Set Up the Game Structure

```rust
use windjammer_game_framework::prelude::*;

struct Platformer {
    world: World,
    player: Entity,
    platforms: Vec<Entity>,
    physics: PhysicsWorld,
}
```

### 2. Create the Player

```rust
impl Platformer {
    fn new() -> Self {
        let mut world = World::new();
        let mut physics = PhysicsWorld::new(Vec2::new(0.0, -9.81));
        
        // Create player entity
        let player = world.spawn()
            .with(Transform2D::new(Vec2::new(100.0, 300.0)))
            .with(Sprite::new(32.0, 32.0, [0.0, 0.5, 1.0, 1.0]))
            .build();
        
        // Add physics body for player
        let player_body = physics.add_dynamic_body(Vec2::new(100.0, 300.0));
        physics.add_collider(player_body, Collider::box_collider(16.0, 16.0));
        
        Self {
            world,
            player,
            platforms: Vec::new(),
            physics,
        }
    }
}
```

### 3. Add Platforms

```rust
fn add_platform(&mut self, x: f32, y: f32, width: f32, height: f32) {
    let platform = self.world.spawn()
        .with(Transform2D::new(Vec2::new(x, y)))
        .with(Sprite::new(width, height, [0.5, 0.5, 0.5, 1.0]))
        .build();
    
    let platform_body = self.physics.add_static_body(Vec2::new(x, y));
    self.physics.add_collider(platform_body, Collider::box_collider(width / 2.0, height / 2.0));
    
    self.platforms.push(platform);
}
```

### 4. Handle Input

```rust
fn handle_input(&mut self, input: &Input) {
    const MOVE_SPEED: f32 = 200.0;
    const JUMP_FORCE: f32 = 500.0;
    
    let mut velocity = Vec2::ZERO;
    
    if input.is_key_down(KeyCode::Left) || input.is_key_down(KeyCode::A) {
        velocity.x = -MOVE_SPEED;
    }
    if input.is_key_down(KeyCode::Right) || input.is_key_down(KeyCode::D) {
        velocity.x = MOVE_SPEED;
    }
    
    if input.is_key_pressed(KeyCode::Space) || input.is_key_pressed(KeyCode::Up) {
        // Apply jump force (check if grounded first in real implementation)
        velocity.y = JUMP_FORCE;
    }
    
    // Apply velocity to player physics body
    // self.physics.set_velocity(player_body, velocity);
}
```

### 5. Update Loop

```rust
impl GameLoop for Platformer {
    fn update(&mut self, delta: f32) {
        // Step physics
        self.physics.step(delta);
        
        // Update transforms from physics
        // Sync physics bodies with entity transforms
    }
    
    fn render(&mut self, ctx: &mut RenderContext) {
        ctx.clear([0.2, 0.6, 1.0, 1.0]); // Sky blue background
        
        // Render platforms
        for &platform in &self.platforms {
            if let Some(transform) = self.world.get_component::<Transform2D>(platform) {
                if let Some(sprite) = self.world.get_component::<Sprite>(platform) {
                    ctx.draw_sprite(sprite, transform);
                }
            }
        }
        
        // Render player
        if let Some(transform) = self.world.get_component::<Transform2D>(self.player) {
            if let Some(sprite) = self.world.get_component::<Sprite>(self.player) {
                ctx.draw_sprite(sprite, transform);
            }
        }
    }
}
```

### 6. Run the Game

```rust
fn main() {
    let mut game = Platformer::new();
    
    // Add some platforms
    game.add_platform(200.0, 100.0, 400.0, 20.0); // Ground
    game.add_platform(100.0, 200.0, 150.0, 20.0); // Platform 1
    game.add_platform(400.0, 300.0, 150.0, 20.0); // Platform 2
    
    // Run the game
    windjammer_game_framework::run(game).expect("Failed to run game");
}
```

## Next Steps

1. Add animation for player movement
2. Implement ground detection for proper jumping
3. Add collectibles and enemies
4. Create multiple levels
5. Add sound effects and music

## Running the Example

```bash
cargo run --example platformer_simple -p windjammer-game-framework
```

## See Also

- [Physics Documentation](../docs/PHYSICS.md)
- [ECS Guide](../docs/ECS.md)
- [Input Handling](../docs/INPUT.md)


