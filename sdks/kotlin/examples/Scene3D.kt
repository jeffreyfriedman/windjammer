/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with the Windjammer Kotlin SDK.
 * 
 * Run with: gradle run -PmainClass=dev.windjammer.examples.Scene3D
 */

package dev.windjammer.examples

import dev.windjammer.sdk.*

fun main() {
    println("=== Windjammer 3D Scene Demo (Kotlin) ===")

    // Create 3D application
    val app = App()

    // Setup system
    app.addStartupSystem {
        println("\n[Setup] Creating 3D scene...")

        // Create 3D camera
        val camera = Camera3D(
            position = Vec3(0f, 5f, 10f),
            lookAt = Vec3(0f, 0f, 0f),
            fov = 60.0f
        )
        println("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)")

        // Create meshes
        val cube = Mesh.cube(1.0f)
        println("  - Cube mesh (size=1.0)")

        val sphere = Mesh.sphere(1.0f, 32)
        println("  - Sphere mesh (radius=1.0, subdivisions=32)")

        val plane = Mesh.plane(10.0f)
        println("  - Plane mesh (size=10.0)")

        // Create materials
        val material = Material(
            albedo = Color(0.8f, 0.2f, 0.2f, 1.0f),
            metallic = 0.5f,
            roughness = 0.5f
        )
        println("  - PBR Material (red, metallic=0.5, roughness=0.5)")

        // Create light
        val light = PointLight(
            position = Vec3(5f, 5f, 5f),
            color = Color(1f, 1f, 1f, 1f),
            intensity = 1000.0f
        )
        println("  - Point Light at (5, 5, 5) intensity=1000")

        println("[Setup] Scene ready!")
    }

    // Update system
    app.addSystem { time: Time ->
        // This would rotate meshes each frame
    }

    println("3D application configured!")
    println("- Camera: Perspective (60° FOV)")
    println("- Rendering: Deferred PBR")
    println("- Lighting: Point Light")
    println()

    // Run the application
    app.run()
}

