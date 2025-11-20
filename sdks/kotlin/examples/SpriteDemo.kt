/**
 * 2D Sprite Demo
 * 
 * Demonstrates 2D sprite rendering with the Windjammer Kotlin SDK.
 * 
 * Run with: gradle run -PmainClass=dev.windjammer.examples.SpriteDemo
 */

package dev.windjammer.examples

import dev.windjammer.sdk.*

fun main() {
    println("=== Windjammer 2D Sprite Demo (Kotlin) ===")

    // Create 2D application
    val app = App()

    // Setup system
    app.addStartupSystem {
        println("\n[Setup] Creating 2D scene...")

        // Create camera
        val camera = Camera2D(Vec2(0f, 0f), 1.0f)
        println("  - $camera")

        // Create sprites
        val sprite1 = Sprite(
            texture = "player.png",
            position = Vec2(0f, 0f),
            size = Vec2(64f, 64f)
        )
        println("  - Sprite 'player.png' at (0, 0) size=(64, 64)")

        val sprite2 = Sprite(
            texture = "enemy.png",
            position = Vec2(100f, 100f),
            size = Vec2(48f, 48f)
        )
        println("  - Sprite 'enemy.png' at (100, 100) size=(48, 48)")

        println("[Setup] Scene ready!")
    }

    // Update system
    app.addSystem {
        // This would rotate sprites each frame
    }

    println("2D application configured!")
    println("- Camera: Orthographic")
    println("- Sprites: Enabled")
    println("- Physics: 2D")
    println()

    // Run the application
    app.run()
}

