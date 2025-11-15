fn main() {
    #[cfg(feature = "tauri")]
    tauri_build::build();
    
    // For desktop-only builds, we don't need tauri_build
    #[cfg(not(feature = "tauri"))]
    {
        // No-op for desktop builds
    }
}
