// Build script to set Windows stack size for ALL executables (main + tests)
// This runs for EVERY binary being built, including test executables

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let profile = std::env::var("PROFILE").unwrap_or_default();
        let crate_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default();

        // 64MB stack - if this doesn't work, problem is structural
        let stack_size = 67108864;

        eprintln!("========================================");
        eprintln!("BUILD SCRIPT: Setting Windows stack size");
        eprintln!("  Package: {}", crate_name);
        eprintln!("  Profile: {}", profile);
        eprintln!("  Stack: {}MB", stack_size / 1048576);
        eprintln!("========================================");

        match target_env.as_str() {
            "msvc" => {
                println!("cargo:rustc-link-arg=/STACK:{}", stack_size);
                println!("cargo:rustc-link-arg=/DEBUG"); // Include debug info
            }
            "gnu" => {
                println!("cargo:rustc-link-arg=-Wl,--stack,{}", stack_size);
            }
            _ => {
                eprintln!("Warning: Unknown target environment: {}", target_env);
            }
        }
    }
}
