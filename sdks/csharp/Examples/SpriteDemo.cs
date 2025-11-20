/**
 * 2D Sprite Demo
 * 
 * Demonstrates 2D sprite rendering with the Windjammer C# SDK.
 * 
 * Run with: dotnet run --project Examples/SpriteDemo.cs
 */

using System;
using Windjammer.SDK;

namespace Windjammer.Examples
{
    class SpriteDemo
    {
        static void Main(string[] args)
        {
            Console.WriteLine("=== Windjammer 2D Sprite Demo (C#) ===");

            // Create 2D application
            var app = new App();

            // Setup system
            app.AddStartupSystem(() =>
            {
                Console.WriteLine("\n[Setup] Creating 2D scene...");

                // Create camera
                var camera = new Camera2D(new Vector2(0, 0), 1.0f);
                Console.WriteLine($"  - {camera}");

                // Create sprites
                var sprite1 = new Sprite
                {
                    Texture = "player.png",
                    Position = new Vector2(0, 0),
                    Size = new Vector2(64, 64)
                };
                Console.WriteLine("  - Sprite 'player.png' at (0, 0) size=(64, 64)");

                var sprite2 = new Sprite
                {
                    Texture = "enemy.png",
                    Position = new Vector2(100, 100),
                    Size = new Vector2(48, 48)
                };
                Console.WriteLine("  - Sprite 'enemy.png' at (100, 100) size=(48, 48)");

                Console.WriteLine("[Setup] Scene ready!");
            });

            // Update system
            app.AddSystem(() =>
            {
                // This would rotate sprites each frame
            });

            Console.WriteLine("2D application configured!");
            Console.WriteLine("- Camera: Orthographic");
            Console.WriteLine("- Sprites: Enabled");
            Console.WriteLine("- Physics: 2D");
            Console.WriteLine();

            // Run the application
            app.Run();
        }
    }
}

