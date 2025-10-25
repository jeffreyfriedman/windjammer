#[cfg(feature = "audio")]
use game_host::audio::AudioPlayer;

fn main() {
    println!("=== Windjammer Audio Test ===\n");

    #[cfg(feature = "audio")]
    {
        match AudioPlayer::new() {
            Ok(mut player) => {
                println!("✅ Audio system initialized");
                player.set_master_volume(0.5);
                println!("✅ Volume set to 50%");
                println!("✅ Audio system ready!");
            }
            Err(e) => {
                println!("❌ Audio system failed: {}", e);
            }
        }
    }

    #[cfg(not(feature = "audio"))]
    {
        println!("⚠️  Audio feature not enabled.");
        println!("Run with: cargo run --example audio_test -p game-host --features audio");
    }
}
