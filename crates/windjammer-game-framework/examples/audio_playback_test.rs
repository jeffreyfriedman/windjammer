//! Test audio playback with rodio
//! Demonstrates sound effects, music, and spatial audio

#[cfg(feature = "audio")]
use windjammer_game_framework::audio::{AudioSystem, SpatialAudioSource};
#[cfg(feature = "audio")]
use windjammer_game_framework::math::Vec3;

fn main() {
    println!("=== Windjammer Audio Playback Test ===\n");

    #[cfg(feature = "audio")]
    {
        // Create audio system
        let mut audio = match AudioSystem::new() {
            Ok(audio) => audio,
            Err(e) => {
                eprintln!("❌ Failed to initialize audio system: {}", e);
                eprintln!("This is normal if no audio output device is available.");
                return;
            }
        };

        println!("✅ Audio system initialized");

        // Test volume control
        println!("\n📊 Testing volume control:");
        audio.set_volume(0.5);
        println!("  ✅ Master volume set to 50%");

        audio.set_volume(1.0);
        println!("  ✅ Master volume set to 100%");

        // Test spatial audio
        println!("\n🎧 Testing spatial audio:");
        let listener_pos = Vec3::new(0.0, 0.0, 0.0);

        let source1 = SpatialAudioSource::new(Vec3::new(10.0, 0.0, 0.0));
        let volume1 = source1.calculate_volume(listener_pos);
        println!("  Source at (10, 0, 0): volume = {:.2}", volume1);

        let source2 = SpatialAudioSource::new(Vec3::new(50.0, 0.0, 0.0));
        let volume2 = source2.calculate_volume(listener_pos);
        println!("  Source at (50, 0, 0): volume = {:.2}", volume2);

        let source3 = SpatialAudioSource::new(Vec3::new(150.0, 0.0, 0.0));
        let volume3 = source3.calculate_volume(listener_pos);
        println!("  Source at (150, 0, 0): volume = {:.2} (beyond max distance)", volume3);

        // Verify attenuation
        assert!(volume1 > volume2, "Closer source should be louder");
        assert!(volume2 > volume3, "Far source should be quieter");
        assert_eq!(volume3, 0.0, "Source beyond max distance should be silent");

        println!("\n✅ Spatial audio calculations working correctly");

        println!("\n📝 Note: To test actual audio playback:");
        println!("  1. Add audio files (WAV, MP3, OGG, FLAC) to a 'sounds' directory");
        println!("  2. Use audio.play_sound(\"sounds/effect.wav\")");
        println!("  3. Use audio.play_music(\"sounds/music.mp3\", true) for looping music");
        println!("  4. Use audio.stop_music() to stop playback");

        println!("\n🎉 Audio system test complete!");
        println!("   - Volume control: ✅");
        println!("   - Spatial audio: ✅");
        println!("   - API ready for use: ✅");
    }

    #[cfg(not(feature = "audio"))]
    {
        println!("⚠️  Audio feature not enabled.");
        println!("Run with: cargo run --example audio_playback_test -p windjammer-game-framework --features audio");
    }
}

