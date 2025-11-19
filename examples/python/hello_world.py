#!/usr/bin/env python3
"""
Windjammer Python SDK - Hello World Example

A minimal Windjammer application that demonstrates:
- App creation and lifecycle
- Basic game loop
- System registration
"""

import sys
sys.path.insert(0, '../../sdks/python/generated')

from app import App
from world import World
from entity import Entity
import time as time_module

def main():
    """Main entry point"""
    print("🎮 Windjammer Python SDK - Hello World")
    print("=" * 50)
    
    # Create application
    app = App()
    print("✓ Created App")
    
    # Create world
    world = World()
    print("✓ Created World")
    
    # Create entities
    player = Entity()
    print(f"✓ Created Player Entity (ID: {player.id if hasattr(player, 'id') else 'N/A'})")
    
    # Add game loop system
    def game_loop():
        """Game update loop"""
        # In a real game, this would be called every frame
        pass
    
    app.add_system(game_loop)
    print("✓ Added game loop system")
    
    print("\n🎉 Hello World complete!")
    print("Note: This is a stub example. Full implementation requires FFI integration.")

if __name__ == '__main__':
    main()

