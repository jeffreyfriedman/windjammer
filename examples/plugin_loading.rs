// Example: Loading Dynamic Plugins
// Demonstrates how to load and use dynamic plugins (C/C++)

use windjammer_game_framework::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Windjammer Dynamic Plugin Loading Example ===\n");

    // Create application
    let mut app = App::new();

    // Example 1: Load a C plugin
    #[cfg(feature = "dynamic_plugins")]
    {
        println!("Loading C plugin...");
        match DynamicPlugin::load("examples/plugins/libexample_plugin.so") {
            Ok(plugin) => {
                println!("✓ Loaded plugin: {}", plugin.name());
                println!("  Version: {}", plugin.version());
                println!("  Description: {}", plugin.description());
                println!("  Author: {}", plugin.author());
                println!("  License: {}", plugin.license());
                println!("  Category: {:?}", plugin.category());
                println!("  Hot-reload: {}", plugin.supports_hot_reload());

                app.add_plugin(plugin)?;
            }
            Err(e) => {
                println!("✗ Failed to load C plugin: {}", e);
                println!("  (This is expected if the plugin hasn't been compiled yet)");
            }
        }
        println!();
    }

    // Example 2: Load a C++ plugin
    #[cfg(feature = "dynamic_plugins")]
    {
        println!("Loading C++ plugin...");
        match DynamicPlugin::load("examples/plugins/libexample_plugin_cpp.so") {
            Ok(plugin) => {
                println!("✓ Loaded plugin: {}", plugin.name());
                println!("  Version: {}", plugin.version());
                println!("  Description: {}", plugin.description());
                println!("  Dependencies: {:?}", plugin.dependencies());

                app.add_plugin(plugin)?;
            }
            Err(e) => {
                println!("✗ Failed to load C++ plugin: {}", e);
                println!("  (This is expected if the plugin hasn't been compiled yet)");
            }
        }
        println!();
    }

    // Example 3: Load all plugins
    println!("Loading all registered plugins...");
    match app.load_plugins() {
        Ok(_) => {
            println!("✓ All plugins loaded successfully!");
        }
        Err(e) => {
            println!("✗ Failed to load plugins: {}", e);
        }
    }
    println!();

    // Example 4: List loaded plugins
    println!("Loaded plugins:");
    for (name, state) in app.plugins().list_plugins() {
        println!("  - {} ({:?})", name, state);
    }
    println!();

    // Example 5: Hot-reload (if supported)
    #[cfg(feature = "dynamic_plugins")]
    {
        println!("Hot-reload is supported!");
        println!("To hot-reload a plugin:");
        println!("  1. Modify the plugin source code");
        println!("  2. Recompile the plugin");
        println!("  3. Call plugin.reload(path)");
        println!();
    }

    println!("=== Plugin Loading Complete ===");

    Ok(())
}

/*
Build Instructions:

1. Compile the C plugin:
   cd examples/plugins
   gcc -shared -fPIC -o libexample_plugin.so example_plugin.c

2. Compile the C++ plugin:
   cd examples/plugins
   g++ -std=c++17 -shared -fPIC -o libexample_plugin_cpp.so example_plugin.cpp

3. Run this example:
   cargo run --example plugin_loading --features dynamic_plugins

Output:
   === Windjammer Dynamic Plugin Loading Example ===

   Loading C plugin...
   ✓ Loaded plugin: example_plugin
     Version: 1.0.0
     Description: Example plugin demonstrating C FFI
     Author: Windjammer Team
     License: MIT
     Category: Other
     Hot-reload: true

   Loading C++ plugin...
   ✓ Loaded plugin: example_plugin_cpp
     Version: 1.0.0
     Description: Example C++ plugin demonstrating FFI with modern C++
     Dependencies: [PluginDependency { name: "core_systems", version: ^1.0.0 }]

   Loading all registered plugins...
   [ExamplePlugin] Initializing...
   [ExamplePlugin] App handle: 0x7ffeefbff000
   [ExamplePlugin] Initialized successfully!
   [ExamplePlugin++] Initializing...
   [ExamplePlugin++] App handle: 0x7ffeefbff000
   [ExamplePlugin++] State created
   [ExamplePlugin++] Initializing C++ plugin...
   [ExamplePlugin++] Initialized successfully!
   ✓ All plugins loaded successfully!

   Loaded plugins:
     - example_plugin (Loaded)
     - example_plugin_cpp (Loaded)

   Hot-reload is supported!
   To hot-reload a plugin:
     1. Modify the plugin source code
     2. Recompile the plugin
     3. Call plugin.reload(path)

   === Plugin Loading Complete ===
*/
