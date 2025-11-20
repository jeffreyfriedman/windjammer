/**
 * 2D Sprite Demo
 * 
 * Demonstrates 2D sprite rendering with the Windjammer C++ SDK.
 * 
 * Build with: cmake --build build && ./build/examples/sprite_demo
 */

#include <windjammer/windjammer.hpp>
#include <iostream>

using namespace windjammer;

int main() {
    std::cout << "=== Windjammer 2D Sprite Demo (C++) ===" << std::endl;
    
    // Create 2D application
    App app;
    
    // Setup system
    app.add_startup_system([]() {
        std::cout << "\n[Setup] Creating 2D scene..." << std::endl;
        
        // Create camera
        auto camera = Camera2D{Vec2{0.0f, 0.0f}, 1.0f};
        std::cout << "  - " << camera << std::endl;
        
        // Create sprites
        auto sprite1 = Sprite{
            .texture = "player.png",
            .position = Vec2{0.0f, 0.0f},
            .size = Vec2{64.0f, 64.0f}
        };
        std::cout << "  - Sprite 'player.png' at (0, 0) size=(64, 64)" << std::endl;
        
        auto sprite2 = Sprite{
            .texture = "enemy.png",
            .position = Vec2{100.0f, 100.0f},
            .size = Vec2{48.0f, 48.0f}
        };
        std::cout << "  - Sprite 'enemy.png' at (100, 100) size=(48, 48)" << std::endl;
        
        std::cout << "[Setup] Scene ready!" << std::endl;
    });
    
    // Update system
    app.add_system([]() {
        // This would rotate sprites each frame
    });
    
    std::cout << "2D application configured!" << std::endl;
    std::cout << "- Camera: Orthographic" << std::endl;
    std::cout << "- Sprites: Enabled" << std::endl;
    std::cout << "- Physics: 2D" << std::endl;
    std::cout << std::endl;
    
    // Run the application
    app.run();
    
    return 0;
}

