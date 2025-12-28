// Build script to ensure Windows stack size is set correctly
// This provides an additional layer beyond .cargo/config.toml

fn main() {
    // Set stack size for Windows targets
    // 16MB stack to prevent overflow during Drop of deep AST structures
    // Debug builds especially need this due to lack of optimization

    println!("cargo:rerun-if-changed=build.rs");

    // Check if we're compiling for Windows (not if we're ON Windows)
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

        match target_env.as_str() {
            "msvc" => {
                // MSVC linker syntax
                println!("cargo:rustc-link-arg=/STACK:16777216");
                eprintln!("Setting Windows MSVC stack size to 16MB");
            }
            "gnu" => {
                // MinGW linker syntax
                println!("cargo:rustc-link-arg=-Wl,--stack,16777216");
                eprintln!("Setting Windows GNU stack size to 16MB");
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
