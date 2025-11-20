/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with the Windjammer Java SDK.
 * 
 * Run with: mvn compile exec:java -Dexec.mainClass="dev.windjammer.examples.Scene3D"
 */

package dev.windjammer.examples;

import dev.windjammer.sdk.*;

public class Scene3D {
    public static void main(String[] args) {
        System.out.println("=== Windjammer 3D Scene Demo (Java) ===");

        // Create 3D application
        var app = new App();

        // Setup system
        app.addStartupSystem(() -> {
            System.out.println("\n[Setup] Creating 3D scene...");

            // Create 3D camera
            var camera = new Camera3D(
                new Vec3(0, 5, 10),
                new Vec3(0, 0, 0),
                60.0f
            );
            System.out.println("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)");

            // Create meshes
            var cube = Mesh.cube(1.0f);
            System.out.println("  - Cube mesh (size=1.0)");

            var sphere = Mesh.sphere(1.0f, 32);
            System.out.println("  - Sphere mesh (radius=1.0, subdivisions=32)");

            var plane = Mesh.plane(10.0f);
            System.out.println("  - Plane mesh (size=10.0)");

            // Create materials
            var material = new Material(
                new Color(0.8f, 0.2f, 0.2f, 1.0f),
                0.5f, // metallic
                0.5f  // roughness
            );
            System.out.println("  - PBR Material (red, metallic=0.5, roughness=0.5)");

            // Create light
            var light = new PointLight(
                new Vec3(5, 5, 5),
                new Color(1, 1, 1, 1),
                1000.0f
            );
            System.out.println("  - Point Light at (5, 5, 5) intensity=1000");

            System.out.println("[Setup] Scene ready!");
        });

        // Update system
        app.addSystem((Time time) -> {
            // This would rotate meshes each frame
        });

        System.out.println("3D application configured!");
        System.out.println("- Camera: Perspective (60° FOV)");
        System.out.println("- Rendering: Deferred PBR");
        System.out.println("- Lighting: Point Light");
        System.out.println();

        // Run the application
        app.run();
    }
}

