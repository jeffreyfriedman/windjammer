#[cfg(feature = "audio")]
use game_host_framework::audio::AudioSystem;

fn main() {
    println!("=== Windjammer Audio Test ===\n");

    #[cfg(feature = "audio")]
    {
        let audio_system = AudioSystem::new();
        println!("✅ Audio system initialized");
        println!("✅ Audio system ready!");
        drop(audio_system); // Ensure it's used
    }

    #[cfg(not(feature = "audio"))]
    {
        println!("⚠️  Audio feature not enabled.");
        println!("Run with: cargo run --example audio_test -p game-host --features audio");
    }
}
