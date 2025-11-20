#!/usr/bin/env swift
/**
 * 2D Sprite Demo
 * 
 * Demonstrates 2D sprite rendering with the Windjammer Swift SDK.
 * 
 * Run with: swift examples/SpriteDemo.swift
 */

import Foundation

print("=== Windjammer 2D Sprite Demo (Swift) ===")

// Create 2D application
let app = App()

// Setup system
app.addStartupSystem {
    print("\n[Setup] Creating 2D scene...")
    
    // Create camera
    let camera = Camera2D(
        position: Vec2(x: 0, y: 0),
        zoom: 1.0
    )
    print("  - \(camera)")
    
    // Create sprites
    let sprite1 = Sprite(
        texture: "player.png",
        position: Vec2(x: 0, y: 0),
        size: Vec2(x: 64, y: 64)
    )
    print("  - Sprite 'player.png' at (0, 0) size=(64, 64)")
    
    let sprite2 = Sprite(
        texture: "enemy.png",
        position: Vec2(x: 100, y: 100),
        size: Vec2(x: 48, y: 48)
    )
    print("  - Sprite 'enemy.png' at (100, 100) size=(48, 48)")
    
    print("[Setup] Scene ready!")
}

// Update system
app.addSystem {
    // This would rotate sprites each frame
}

print("2D application configured!")
print("- Camera: Orthographic")
print("- Sprites: Enabled")
print("- Physics: 2D")
print()

// Run the application
app.run()

