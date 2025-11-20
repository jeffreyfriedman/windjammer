/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
 * 
 * Build with: cmake --build build && ./build/examples/3d_scene
 */

#include <windjammer/windjammer.hpp>

using namespace windjammer;

int main() {
    App app;
    
    app.add_startup_system([]() {
        // Camera
        Camera3D{
            .position = Vec3{0.0f, 5.0f, 10.0f},
            .look_at = Vec3{0.0f, 0.0f, 0.0f},
            .fov = 60.0f
        };
        
        // Lights
        PointLight{ Vec3{5.0f, 5.0f, 5.0f}, Color{1.0f, 0.8f, 0.6f, 1.0f}, 2000.0f };
        PointLight{ Vec3{-5.0f, 5.0f, 5.0f}, Color{0.6f, 0.8f, 1.0f, 1.0f}, 1500.0f };
        PointLight{ Vec3{0.0f, 10.0f, -5.0f}, Color{1.0f, 1.0f, 1.0f, 1.0f}, 1000.0f };
        
        // Meshes with PBR materials
        Mesh::cube(1.0f).with_material(Material{
            .albedo = Color{0.8f, 0.2f, 0.2f, 1.0f},
            .metallic = 0.8f,
            .roughness = 0.2f,
            .emissive = Color{0.5f, 0.1f, 0.1f, 1.0f}
        });
        
        Mesh::sphere(1.0f, 32).with_material(Material{
            .albedo = Color{0.2f, 0.2f, 0.8f, 1.0f},
            .metallic = 0.5f,
            .roughness = 0.5f,
            .emissive = Color{0.1f, 0.1f, 0.5f, 1.0f}
        });
        
        Mesh::plane(10.0f).with_material(Material{
            .albedo = Color{0.3f, 0.3f, 0.3f, 1.0f},
            .metallic = 0.0f,
            .roughness = 0.9f
        });
        
        // Post-processing
        auto post = PostProcessing{};
        post.enable_hdr(true);
        post.set_bloom(BloomSettings{ .threshold = 1.0f, .intensity = 0.8f, .radius = 4.0f, .soft_knee = 0.5f });
        post.set_ssao(SSAOSettings{ .radius = 0.5f, .intensity = 1.5f, .bias = 0.025f, .samples = 16 });
        post.set_tone_mapping(ToneMappingMode::ACES, 1.2f);
        post.set_color_grading(ColorGrading{ .temperature = 0.1f, .tint = 0.0f, .saturation = 1.2f, .contrast = 1.1f });
    });
    
    app.add_system([](const Time& time) {
        // Rotate objects for dynamic lighting
    });
    
    app.run();
    return 0;
}
