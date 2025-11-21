// Example Windjammer Plugin in C
// Demonstrates the C FFI plugin interface

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>

// ============================================================================
// Windjammer Plugin API (C Header)
// ============================================================================

// Opaque handle to Windjammer App
typedef struct WjApp WjApp;

// Plugin error codes
typedef enum {
    WJ_OK = 0,
    WJ_INVALID_PARAMETER = 1,
    WJ_PLUGIN_NOT_FOUND = 2,
    WJ_DEPENDENCY_ERROR = 3,
    WJ_ALREADY_LOADED = 4,
    WJ_LOAD_FAILED = 5,
    WJ_UNLOAD_FAILED = 6,
    WJ_VERSION_MISMATCH = 7,
    WJ_CIRCULAR_DEPENDENCY = 8,
} WjPluginErrorCode;

// Plugin categories
typedef enum {
    WJ_CATEGORY_RENDERING = 0,
    WJ_CATEGORY_PHYSICS = 1,
    WJ_CATEGORY_AUDIO = 2,
    WJ_CATEGORY_AI = 3,
    WJ_CATEGORY_EDITOR = 4,
    WJ_CATEGORY_ASSETS = 5,
    WJ_CATEGORY_NETWORKING = 6,
    WJ_CATEGORY_PLATFORM = 7,
    WJ_CATEGORY_OTHER = 8,
} WjPluginCategory;

// Plugin metadata
typedef struct {
    const char* name;
    const char* version;
    const char* description;
    const char* author;
    const char* license;
    WjPluginCategory category;
    bool supports_hot_reload;
} WjPluginInfo;

// Plugin dependency
typedef struct {
    const char* name;
    const char* version;
} WjPluginDependency;

// ============================================================================
// Plugin Implementation
// ============================================================================

// Plugin metadata
#ifdef _WIN32
__declspec(dllexport)
#endif
WjPluginInfo wj_plugin_info(void) {
    WjPluginInfo info = {
        .name = "example_plugin",
        .version = "1.0.0",
        .description = "Example plugin demonstrating C FFI",
        .author = "Windjammer Team",
        .license = "MIT",
        .category = WJ_CATEGORY_OTHER,
        .supports_hot_reload = true,
    };
    return info;
}

// Plugin dependencies (optional)
#ifdef _WIN32
__declspec(dllexport)
#endif
const WjPluginDependency* wj_plugin_dependencies(size_t* out_count) {
    // No dependencies for this example
    *out_count = 0;
    return NULL;
}

// Plugin initialization
#ifdef _WIN32
__declspec(dllexport)
#endif
WjPluginErrorCode wj_plugin_init(WjApp* app) {
    printf("[ExamplePlugin] Initializing...\n");
    printf("[ExamplePlugin] App handle: %p\n", (void*)app);
    
    // Add systems, resources, etc. here
    // For now, just print a message
    
    printf("[ExamplePlugin] Initialized successfully!\n");
    return WJ_OK;
}

// Plugin cleanup
#ifdef _WIN32
__declspec(dllexport)
#endif
WjPluginErrorCode wj_plugin_cleanup(WjApp* app) {
    printf("[ExamplePlugin] Cleaning up...\n");
    printf("[ExamplePlugin] App handle: %p\n", (void*)app);
    
    // Cleanup resources here
    
    printf("[ExamplePlugin] Cleaned up successfully!\n");
    return WJ_OK;
}

// ============================================================================
// Build Instructions
// ============================================================================

/*
Linux/macOS:
    gcc -shared -fPIC -o libexample_plugin.so example_plugin.c

Windows (MSVC):
    cl /LD example_plugin.c /Fe:example_plugin.dll

Windows (MinGW):
    gcc -shared -o example_plugin.dll example_plugin.c

Usage:
    // In Rust
    use windjammer_game_framework::prelude::*;
    
    let mut app = App::new();
    let plugin = DynamicPlugin::load("libexample_plugin.so")?;
    app.add_plugin(plugin)?;
    app.load_plugins()?;
*/

