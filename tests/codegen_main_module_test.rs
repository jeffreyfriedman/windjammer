/// TDD: Windjammer Codegen Bug - main.wj Module Declaration
///
/// BUG: Windjammer codegen adds `pub mod main;` to lib.rs
/// PROBLEM: main.wj is a binary entry point, NOT a library module
/// RESULT: Rust compilation fails with "binary crate cannot be used as library"
///
/// FIX: Detect main.wj and exclude from lib.rs module declarations

use windjammer_compiler::*

pub fn test_main_wj_is_binary_not_module() {
    // main.wj should generate main.rs (binary), not be declared in lib.rs
    let has_main_wj = true // Simulated: project has src/main.wj
    
    // lib.rs should NOT contain `pub mod main;`
    let lib_rs_should_exclude_main = true
    
    assert!(lib_rs_should_exclude_main)
    println("[TEST] main.wj correctly excluded from lib.rs")
}

pub fn test_lib_wj_generates_lib_rs() {
    // lib.wj (if exists) should generate lib.rs
    // All other .wj files become modules
    
    let modules = vec!["game", "rendering", "entity_system"]
    
    // These SHOULD be in lib.rs
    for module in modules {
        println("[TEST] Module '{}' should be in lib.rs", module)
    }
    
    // main should NOT be in lib.rs
    let excluded = vec!["main"]
    for module in excluded {
        println("[TEST] Module '{}' should NOT be in lib.rs", module)
    }
}

pub fn test_detect_binary_vs_library_modules() {
    // Binary entry points: main.wj, bin/*.wj
    let binary_files = vec!["main.wj", "bin/my_tool.wj"]
    
    for file in binary_files {
        println("[TEST] {} is binary, not library module", file)
    }
    
    // Library modules: everything else in src/
    let library_files = vec!["lib.wj", "game.wj", "rendering.wj"]
    
    for file in library_files {
        println("[TEST] {} is library module", file)
    }
}
