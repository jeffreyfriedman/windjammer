#!/usr/bin/env python3
"""
Windjammer Python SDK - 2D Platformer Example

Demonstrates:
- 2D physics (RigidBody2D)
- Sprite rendering
- Input handling
- Camera system
- Entity management
"""

import sys
sys.path.insert(0, '../../sdks/python/generated')

from app import App
from world import World
from entity import Entity
from sprite import Sprite
from camera2d import Camera2D
from rigidbody2d import RigidBody2D
from rigidbodytype import RigidBodyType
from input import Input
from key import Key
from vec2 import Vec2
from color import Color
from transform2d import Transform2D

def main():
    """2D Platformer game"""
    print("🎮 Windjammer Python SDK - 2D Platformer")
    print("=" * 50)
    
    # Create app and world
    app = App()
    world = World()
    input_system = Input()
    
    # Create player entity (stub - would use world.create_entity() with FFI)
    player = Entity()
    player_transform = Transform2D(Vec2(0.0, 0.0), 0.0, Vec2(1.0, 1.0))
    player_sprite = Sprite("assets/player.png")
    player_sprite.color = Color(1.0, 1.0, 1.0, 1.0)
    player_physics = RigidBody2D(RigidBodyType.Dynamic)
    player_physics.mass = 1.0
    
    print(f"✓ Created Player")
    print(f"  Position: ({player_transform.position.x}, {player_transform.position.y})")
    print(f"  Sprite: {player_sprite.texture_path}")
    print(f"  Physics: Dynamic")
    
    # Create ground entity (stub)
    ground = Entity()
    ground_transform = Transform2D(Vec2(0.0, -5.0), 0.0, Vec2(10.0, 1.0))
    ground_physics = RigidBody2D(RigidBodyType.Static)
    
    print(f"✓ Created Ground")
    print(f"  Position: ({ground_transform.position.x}, {ground_transform.position.y})")
    print(f"  Scale: ({ground_transform.scale.x}, {ground_transform.scale.y})")
    
    # Create camera
    camera = Camera2D()
    camera.position = Vec2(0.0, 0.0)
    camera.zoom = 1.0
    
    print(f"✓ Created Camera")
    print(f"  Position: ({camera.position.x}, {camera.position.y})")
    print(f"  Zoom: {camera.zoom}")
    
    # Game loop system
    def update_system():
        """Update game logic"""
        # Handle input
        if input_system.is_key_held(Key.A):
            # Move left
            force = Vec2(-10.0, 0.0)
            player_physics.apply_force(force)
        
        if input_system.is_key_held(Key.D):
            # Move right
            force = Vec2(10.0, 0.0)
            player_physics.apply_force(force)
        
        if input_system.is_key_pressed(Key.Space):
            # Jump
            impulse = Vec2(0.0, 15.0)
            player_physics.apply_impulse(impulse)
        
        # Camera follows player
        camera.follow(player_transform.position)
    
    app.add_system(update_system)
    
    print("\n🎮 Game Setup Complete!")
    print("\nControls:")
    print("  A/D - Move left/right")
    print("  Space - Jump")
    print("\nNote: This is a stub example. Full implementation requires FFI integration.")

if __name__ == '__main__':
    main()

