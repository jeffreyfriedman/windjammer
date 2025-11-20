/**
 * 2D Sprite Demo
 * 
 * Demonstrates 2D sprite rendering with the Windjammer Java SDK.
 * 
 * Run with: mvn compile exec:java -Dexec.mainClass="dev.windjammer.examples.SpriteDemo"
 */

package dev.windjammer.examples;

import dev.windjammer.sdk.*;

public class SpriteDemo {
    public static void main(String[] args) {
        System.out.println("=== Windjammer 2D Sprite Demo (Java) ===");

        // Create 2D application
        var app = new App();

        // Setup system
        app.addStartupSystem(() -> {
            System.out.println("\n[Setup] Creating 2D scene...");

            // Create camera
            var camera = new Camera2D(new Vec2(0, 0), 1.0f);
            System.out.println("  - " + camera);

            // Create sprites
            var sprite1 = new Sprite("player.png", new Vec2(0, 0), new Vec2(64, 64));
            System.out.println("  - Sprite 'player.png' at (0, 0) size=(64, 64)");

            var sprite2 = new Sprite("enemy.png", new Vec2(100, 100), new Vec2(48, 48));
            System.out.println("  - Sprite 'enemy.png' at (100, 100) size=(48, 48)");

            System.out.println("[Setup] Scene ready!");
        });

        // Update system
        app.addSystem(() -> {
            // This would rotate sprites each frame
        });

        System.out.println("2D application configured!");
        System.out.println("- Camera: Orthographic");
        System.out.println("- Sprites: Enabled");
        System.out.println("- Physics: 2D");
        System.out.println();

        // Run the application
        app.run();
    }
}

