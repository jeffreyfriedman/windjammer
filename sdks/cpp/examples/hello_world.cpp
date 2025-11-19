/**
 * @file hello_world.cpp
 * @brief Hello World Example
 * 
 * The simplest possible Windjammer application in C++.
 * 
 * Build and run:
 *   mkdir build && cd build
 *   cmake ..
 *   cmake --build .
 *   ./examples/hello_world
 */

#include <windjammer/windjammer.hpp>
#include <iostream>

int main() {
    std::cout << "=== Windjammer Hello World (C++) ===\n";
    std::cout << "SDK Version: " << wj::VERSION << "\n\n";
    
    // Create a new application
    wj::App app;
    
    // Add a simple system
    app.add_system([]() {
        std::cout << "Hello from the game loop!\n";
    });
    
    std::cout << "Application created successfully!\n";
    std::cout << "Systems registered: 1\n\n";
    std::cout << "Note: Full app.run() would start the game loop\n";
    std::cout << "For this example, we're just demonstrating SDK setup\n\n";
    
    // Run the application
    app.run();
    
    return 0;
}

