/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
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
            var app = new App();

            app.AddStartupSystem(() =>
            {
                // Camera
                new Camera3D(
                    position: new Vector3(0, 5, 10),
                    lookAt: new Vector3(0, 0, 0),
                    fov: 60.0f
                );

                // Lights
                new PointLight { Position = new Vector3(5, 5, 5), Color = new Color(1.0f, 0.8f, 0.6f, 1.0f), Intensity = 2000.0f };
                new PointLight { Position = new Vector3(-5, 5, 5), Color = new Color(0.6f, 0.8f, 1.0f, 1.0f), Intensity = 1500.0f };
                new PointLight { Position = new Vector3(0, 10, -5), Color = new Color(1, 1, 1, 1), Intensity = 1000.0f };

                // Meshes with PBR materials
                Mesh.Cube(1.0f).WithMaterial(new Material
                {
                    Albedo = new Color(0.8f, 0.2f, 0.2f, 1.0f),
                    Metallic = 0.8f,
                    Roughness = 0.2f,
                    Emissive = new Color(0.5f, 0.1f, 0.1f, 1.0f)
                });

                Mesh.Sphere(1.0f, 32).WithMaterial(new Material
                {
                    Albedo = new Color(0.2f, 0.2f, 0.8f, 1.0f),
                    Metallic = 0.5f,
                    Roughness = 0.5f,
                    Emissive = new Color(0.1f, 0.1f, 0.5f, 1.0f)
                });

                Mesh.Plane(10.0f).WithMaterial(new Material
                {
                    Albedo = new Color(0.3f, 0.3f, 0.3f, 1.0f),
                    Metallic = 0.0f,
                    Roughness = 0.9f
                });

                // Post-processing
                var post = new PostProcessing();
                post.EnableHDR(true);
                post.SetBloom(new BloomSettings { Threshold = 1.0f, Intensity = 0.8f, Radius = 4.0f, SoftKnee = 0.5f });
                post.SetSSAO(new SSAOSettings { Radius = 0.5f, Intensity = 1.5f, Bias = 0.025f, Samples = 16 });
                post.SetToneMapping(ToneMappingMode.ACES, 1.2f);
                post.SetColorGrading(new ColorGrading { Temperature = 0.1f, Tint = 0.0f, Saturation = 1.2f, Contrast = 1.1f });
            });

            app.AddSystem((Time time) =>
            {
                // Rotate objects for dynamic lighting
            });

            app.Run();
        }
    }
}
