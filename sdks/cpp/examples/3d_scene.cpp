/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with the Windjammer C++ SDK.
 * 
 * Build with: cmake --build build && ./build/examples/3d_scene
 */

#include <windjammer/windjammer.hpp>
#include <iostream>

using namespace windjammer;

int main() {
    std::cout << "=== Windjammer 3D Scene Demo (C++) ===" << std::endl;
    
    // Create 3D application
    App app;
    
    // Setup system
    app.add_startup_system([]() {
        std::cout << "\n[Setup] Creating 3D scene..." << std::endl;
        
        // Create 3D camera
        auto camera = Camera3D{
            .position = Vec3{0.0f, 5.0f, 10.0f},
            .look_at = Vec3{0.0f, 0.0f, 0.0f},
            .fov = 60.0f
        };
        std::cout << "  - Camera3D at (0, 5, 10) looking at (0, 0, 0)" << std::endl;
        
        // Create meshes
        auto cube = Mesh::cube(1.0f);
        std::cout << "  - Cube mesh (size=1.0)" << std::endl;
        
        auto sphere = Mesh::sphere(1.0f, 32);
        std::cout << "  - Sphere mesh (radius=1.0, subdivisions=32)" << std::endl;
        
        auto plane = Mesh::plane(10.0f);
        std::cout << "  - Plane mesh (size=10.0)" << std::endl;
        
        // Create materials
        auto material = Material{
            .albedo = Color{0.8f, 0.2f, 0.2f, 1.0f},
            .metallic = 0.5f,
            .roughness = 0.5f
        };
        std::cout << "  - PBR Material (red, metallic=0.5, roughness=0.5)" << std::endl;
        
        // Create light
        auto light = PointLight{
            .position = Vec3{5.0f, 5.0f, 5.0f},
            .color = Color{1.0f, 1.0f, 1.0f, 1.0f},
            .intensity = 1000.0f
        };
        std::cout << "  - Point Light at (5, 5, 5) intensity=1000" << std::endl;
        
        std::cout << "[Setup] Scene ready!" << std::endl;
    });
    
    // Update system
    app.add_system([](const Time& time) {
        // This would rotate meshes each frame
    });
    
    std::cout << "3D application configured!" << std::endl;
    std::cout << "- Camera: Perspective (60° FOV)" << std::endl;
    std::cout << "- Rendering: Deferred PBR" << std::endl;
    std::cout << "- Lighting: Point Light" << std::endl;
    std::cout << std::endl;
    
    // Run the application
    app.run();
    
    return 0;
}

