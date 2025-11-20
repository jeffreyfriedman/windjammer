/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with the Windjammer C# SDK.
 * 
 * Run with: dotnet run --project Examples/3DScene.cs
 */

using System;
using Windjammer.SDK;

namespace Windjammer.Examples
{
    class Scene3D
    {
        static void Main(string[] args)
        {
            Console.WriteLine("=== Windjammer 3D Scene Demo (C#) ===");

            // Create 3D application
            var app = new App();

            // Setup system
            app.AddStartupSystem(() =>
            {
                Console.WriteLine("\n[Setup] Creating 3D scene...");

                // Create 3D camera
                var camera = new Camera3D(
                    position: new Vector3(0, 5, 10),
                    lookAt: new Vector3(0, 0, 0),
                    fov: 60.0f
                );
                Console.WriteLine("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)");

                // Create meshes
                var cube = Mesh.Cube(size: 1.0f);
                Console.WriteLine("  - Cube mesh (size=1.0)");

                var sphere = Mesh.Sphere(radius: 1.0f, subdivisions: 32);
                Console.WriteLine("  - Sphere mesh (radius=1.0, subdivisions=32)");

                var plane = Mesh.Plane(size: 10.0f);
                Console.WriteLine("  - Plane mesh (size=10.0)");

                // Create materials
                var material = new Material
                {
                    Albedo = new Color(0.8f, 0.2f, 0.2f, 1.0f),
                    Metallic = 0.5f,
                    Roughness = 0.5f
                };
                Console.WriteLine("  - PBR Material (red, metallic=0.5, roughness=0.5)");

                // Create light
                var light = new PointLight
                {
                    Position = new Vector3(5, 5, 5),
                    Color = new Color(1, 1, 1, 1),
                    Intensity = 1000.0f
                };
                Console.WriteLine("  - Point Light at (5, 5, 5) intensity=1000");

                Console.WriteLine("[Setup] Scene ready!");
            });

            // Update system
            app.AddSystem((Time time) =>
            {
                // This would rotate meshes each frame
            });

            Console.WriteLine("3D application configured!");
            Console.WriteLine("- Camera: Perspective (60° FOV)");
            Console.WriteLine("- Rendering: Deferred PBR");
            Console.WriteLine("- Lighting: Point Light");
            Console.WriteLine();

            // Run the application
            app.Run();
        }
    }
}

