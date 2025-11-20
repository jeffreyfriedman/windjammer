#!/usr/bin/env python3
"""
2D Sprite Demo

Demonstrates 2D sprite rendering with the Windjammer Python SDK.

Run with: python examples/sprite_demo.py
"""

from windjammer_sdk import App, Vec2, Sprite, Camera2D, Color

def main():
    print("=== Windjammer 2D Sprite Demo (Python) ===")
    
    # Create 2D application
    app = App(title="2D Sprite Demo")
    
    # Setup system
    def setup_2d_scene():
        print("\n[Setup] Creating 2D scene...")
        
        # Create camera
        camera = Camera2D(position=Vec2(0, 0), zoom=1.0)
        print(f"  - {camera}")
        
        # Create sprites
        sprite1 = Sprite(
            texture="player.png",
            position=Vec2(0, 0),
            size=Vec2(64, 64),
            color=Color.white()
        )
        print(f"  - {sprite1}")
        
        sprite2 = Sprite(
            texture="enemy.png",
            position=Vec2(100, 100),
            size=Vec2(48, 48),
            color=Color.red()
        )
        print(f"  - {sprite2}")
        
        print("[Setup] Scene ready!")
    
    app.add_startup_system(setup_2d_scene)
    
    # Update system
    def rotate_sprites():
        # This would rotate sprites each frame
        pass
    
    app.add_system(rotate_sprites)
    
    print("2D application configured!")
    print("- Camera: Orthographic")
    print("- Sprites: Enabled")
    print("- Physics: 2D")
    print()
    
    # Run the application
    app.run()

if __name__ == "__main__":
    main()
