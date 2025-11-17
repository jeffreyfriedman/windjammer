// Example Windjammer Plugin in C++
// Demonstrates the C FFI plugin interface with C++ features

#include <iostream>
#include <string>
#include <vector>
#include <memory>

extern "C" {

// ============================================================================
// Windjammer Plugin API (C Header)
// ============================================================================

// Opaque handle to Windjammer App
struct WjApp;

// Plugin error codes
enum WjPluginErrorCode {
    WJ_OK = 0,
    WJ_INVALID_PARAMETER = 1,
    WJ_PLUGIN_NOT_FOUND = 2,
    WJ_DEPENDENCY_ERROR = 3,
    WJ_ALREADY_LOADED = 4,
    WJ_LOAD_FAILED = 5,
    WJ_UNLOAD_FAILED = 6,
    WJ_VERSION_MISMATCH = 7,
    WJ_CIRCULAR_DEPENDENCY = 8,
};

// Plugin categories
enum WjPluginCategory {
    WJ_CATEGORY_RENDERING = 0,
    WJ_CATEGORY_PHYSICS = 1,
    WJ_CATEGORY_AUDIO = 2,
    WJ_CATEGORY_AI = 3,
    WJ_CATEGORY_EDITOR = 4,
    WJ_CATEGORY_ASSETS = 5,
    WJ_CATEGORY_NETWORKING = 6,
    WJ_CATEGORY_PLATFORM = 7,
    WJ_CATEGORY_OTHER = 8,
};

// Plugin metadata
struct WjPluginInfo {
    const char* name;
    const char* version;
    const char* description;
    const char* author;
    const char* license;
    WjPluginCategory category;
    bool supports_hot_reload;
};

// Plugin dependency
struct WjPluginDependency {
    const char* name;
    const char* version;
};

// ============================================================================
// Plugin Implementation (C++ with RAII)
// ============================================================================

// Plugin state (using C++ features)
class PluginState {
public:
    PluginState() {
        std::cout << "[ExamplePlugin++] State created" << std::endl;
    }
    
    ~PluginState() {
        std::cout << "[ExamplePlugin++] State destroyed" << std::endl;
    }
    
    void initialize() {
        std::cout << "[ExamplePlugin++] Initializing C++ plugin..." << std::endl;
        // Add your initialization logic here
    }
    
    void cleanup() {
        std::cout << "[ExamplePlugin++] Cleaning up C++ plugin..." << std::endl;
        // Add your cleanup logic here
    }
};

// Global plugin state (managed by C++)
static std::unique_ptr<PluginState> g_plugin_state;

// Plugin metadata
#ifdef _WIN32
__declspec(dllexport)
#endif
WjPluginInfo wj_plugin_info(void) {
    static WjPluginInfo info = {
        "example_plugin_cpp",
        "1.0.0",
        "Example C++ plugin demonstrating FFI with modern C++",
        "Windjammer Team",
        "MIT",
        WJ_CATEGORY_OTHER,
        true,
    };
    return info;
}

// Plugin dependencies (optional)
#ifdef _WIN32
__declspec(dllexport)
#endif
const WjPluginDependency* wj_plugin_dependencies(size_t* out_count) {
    // Example: depend on another plugin
    static WjPluginDependency deps[] = {
        {"core_systems", "^1.0.0"},
    };
    *out_count = 1;
    return deps;
}

// Plugin initialization
#ifdef _WIN32
__declspec(dllexport)
#endif
WjPluginErrorCode wj_plugin_init(WjApp* app) {
    try {
        std::cout << "[ExamplePlugin++] Initializing..." << std::endl;
        std::cout << "[ExamplePlugin++] App handle: " << app << std::endl;
        
        // Create plugin state (RAII)
        g_plugin_state = std::make_unique<PluginState>();
        g_plugin_state->initialize();
        
        std::cout << "[ExamplePlugin++] Initialized successfully!" << std::endl;
        return WJ_OK;
    } catch (const std::exception& e) {
        std::cerr << "[ExamplePlugin++] Initialization failed: " << e.what() << std::endl;
        return WJ_LOAD_FAILED;
    }
}

// Plugin cleanup
#ifdef _WIN32
__declspec(dllexport)
#endif
WjPluginErrorCode wj_plugin_cleanup(WjApp* app) {
    try {
        std::cout << "[ExamplePlugin++] Cleaning up..." << std::endl;
        std::cout << "[ExamplePlugin++] App handle: " << app << std::endl;
        
        if (g_plugin_state) {
            g_plugin_state->cleanup();
            g_plugin_state.reset();  // RAII cleanup
        }
        
        std::cout << "[ExamplePlugin++] Cleaned up successfully!" << std::endl;
        return WJ_OK;
    } catch (const std::exception& e) {
        std::cerr << "[ExamplePlugin++] Cleanup failed: " << e.what() << std::endl;
        return WJ_UNLOAD_FAILED;
    }
}

} // extern "C"

// ============================================================================
// Build Instructions
// ============================================================================

/*
Linux/macOS:
    g++ -std=c++17 -shared -fPIC -o libexample_plugin_cpp.so example_plugin.cpp

Windows (MSVC):
    cl /std:c++17 /LD example_plugin.cpp /Fe:example_plugin_cpp.dll

Windows (MinGW):
    g++ -std=c++17 -shared -o example_plugin_cpp.dll example_plugin.cpp

Usage:
    // In Rust
    use windjammer_game_framework::prelude::*;
    
    let mut app = App::new();
    let plugin = DynamicPlugin::load("libexample_plugin_cpp.so")?;
    app.add_plugin(plugin)?;
    app.load_plugins()?;
    
Key Features:
- Modern C++ (C++17) with RAII
- Exception handling
- Smart pointers (unique_ptr)
- C linkage for FFI compatibility
- Cross-platform (Linux, macOS, Windows)
*/

