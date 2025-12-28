// Build script to ensure Windows stack size is set correctly
// This provides an additional layer beyond .cargo/config.toml

fn main() {
    // Set stack size for Windows targets based on optimization level
    // - Debug builds: 16MB (no optimization, deeper call stacks)
    // - Release builds: 8MB (optimizations reduce stack usage)

    println!("cargo:rerun-if-changed=build.rs");

    // Check if we're compiling for Windows (not if we're ON Windows)
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let profile = std::env::var("PROFILE").unwrap_or_default();

        // Choose stack size based on optimization level
        let stack_size = if profile == "release" {
            8388608 // 8MB for release (optimizations enabled)
        } else {
            16777216 // 16MB for debug/test (no optimizations)
        };

        match target_env.as_str() {
            "msvc" => {
                // MSVC linker syntax
                println!("cargo:rustc-link-arg=/STACK:{}", stack_size);
                eprintln!(
                    "Setting Windows MSVC stack size to {}MB (profile: {})",
                    stack_size / 1048576,
                    profile
                );
            }
            "gnu" => {
                // MinGW linker syntax
                println!("cargo:rustc-link-arg=-Wl,--stack,{}", stack_size);
                eprintln!(
                    "Setting Windows GNU stack size to {}MB (profile: {})",
                    stack_size / 1048576,
                    profile
                );
            }
            _ => {
                eprintln!(
                    "Warning: Unknown Windows target environment: {}",
                    target_env
                );
            }
        }
    }
}
