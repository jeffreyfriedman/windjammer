/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
 * 
 * Run with: gradle run -PmainClass=dev.windjammer.examples.Scene3D
 */

package dev.windjammer.examples

import dev.windjammer.sdk.*

fun main() {
    val app = App()

    app.addStartupSystem {
        // Camera
        Camera3D(
            position = Vec3(0f, 5f, 10f),
            lookAt = Vec3(0f, 0f, 0f),
            fov = 60.0f
        )

        // Lights
        PointLight(Vec3(5f, 5f, 5f), Color(1.0f, 0.8f, 0.6f, 1.0f), 2000.0f)
        PointLight(Vec3(-5f, 5f, 5f), Color(0.6f, 0.8f, 1.0f, 1.0f), 1500.0f)
        PointLight(Vec3(0f, 10f, -5f), Color(1f, 1f, 1f, 1f), 1000.0f)

        // Meshes with PBR materials
        Mesh.cube(1.0f).withMaterial(Material(
            albedo = Color(0.8f, 0.2f, 0.2f, 1.0f),
            metallic = 0.8f,
            roughness = 0.2f,
            emissive = Color(0.5f, 0.1f, 0.1f, 1.0f)
        ))

        Mesh.sphere(1.0f, 32).withMaterial(Material(
            albedo = Color(0.2f, 0.2f, 0.8f, 1.0f),
            metallic = 0.5f,
            roughness = 0.5f,
            emissive = Color(0.1f, 0.1f, 0.5f, 1.0f)
        ))

        Mesh.plane(10.0f).withMaterial(Material(
            albedo = Color(0.3f, 0.3f, 0.3f, 1.0f),
            metallic = 0.0f,
            roughness = 0.9f
        ))

        // Post-processing
        val post = PostProcessing()
        post.enableHDR(true)
        post.setBloom(BloomSettings(threshold = 1.0f, intensity = 0.8f, radius = 4.0f, softKnee = 0.5f))
        post.setSSAO(SSAOSettings(radius = 0.5f, intensity = 1.5f, bias = 0.025f, samples = 16))
        post.setToneMapping(ToneMappingMode.ACES, 1.2f)
        post.setColorGrading(ColorGrading(temperature = 0.1f, tint = 0.0f, saturation = 1.2f, contrast = 1.1f))
    }

    app.addSystem { time: Time ->
        // Rotate objects for dynamic lighting
    }

    app.run()
}
