/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
 * 
 * Run with: mvn compile exec:java -Dexec.mainClass="dev.windjammer.examples.Scene3D"
 */

package dev.windjammer.examples;

import dev.windjammer.sdk.*;

public class Scene3D {
    public static void main(String[] args) {
        var app = new App();

        app.addStartupSystem(() -> {
            // Camera
            new Camera3D(
                new Vec3(0, 5, 10),
                new Vec3(0, 0, 0),
                60.0f
            );

            // Lights
            new PointLight(new Vec3(5, 5, 5), new Color(1.0f, 0.8f, 0.6f, 1.0f), 2000.0f);
            new PointLight(new Vec3(-5, 5, 5), new Color(0.6f, 0.8f, 1.0f, 1.0f), 1500.0f);
            new PointLight(new Vec3(0, 10, -5), new Color(1, 1, 1, 1), 1000.0f);

            // Meshes with PBR materials
            Mesh.cube(1.0f).withMaterial(new Material(
                new Color(0.8f, 0.2f, 0.2f, 1.0f),
                0.8f, // metallic
                0.2f, // roughness
                new Color(0.5f, 0.1f, 0.1f, 1.0f) // emissive
            ));

            Mesh.sphere(1.0f, 32).withMaterial(new Material(
                new Color(0.2f, 0.2f, 0.8f, 1.0f),
                0.5f, // metallic
                0.5f, // roughness
                new Color(0.1f, 0.1f, 0.5f, 1.0f) // emissive
            ));

            Mesh.plane(10.0f).withMaterial(new Material(
                new Color(0.3f, 0.3f, 0.3f, 1.0f),
                0.0f, // metallic
                0.9f  // roughness
            ));

            // Post-processing
            var post = new PostProcessing();
            post.enableHDR(true);
            post.setBloom(new BloomSettings(1.0f, 0.8f, 4.0f, 0.5f));
            post.setSSAO(new SSAOSettings(0.5f, 1.5f, 0.025f, 16));
            post.setToneMapping(ToneMappingMode.ACES, 1.2f);
            post.setColorGrading(new ColorGrading(0.1f, 0.0f, 1.2f, 1.1f));
        });

        app.addSystem((Time time) -> {
            // Rotate objects for dynamic lighting
        });

        app.run();
    }
}
