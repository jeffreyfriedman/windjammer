//! Unit tests for Advanced Audio System
//!
//! Tests 3D spatial audio, mixing, buses, and effects.

use windjammer_game_framework::audio_advanced::*;
use windjammer_game_framework::math::Vec3;

// ============================================================================
// AudioEngine Tests
// ============================================================================

#[test]
fn test_audio_engine_creation() {
    let engine = AudioEngine::new();
    assert_eq!(engine.master_volume, 1.0);
    assert_eq!(engine.speed_of_sound, 343.0);
    assert_eq!(engine.doppler_factor, 1.0);
    println!("✅ AudioEngine created");
}

#[test]
fn test_audio_engine_default() {
    let engine = AudioEngine::default();
    assert_eq!(engine.master_volume, 1.0);
    println!("✅ AudioEngine default");
}

#[test]
fn test_default_buses() {
    let engine = AudioEngine::new();
    
    assert!(engine.get_bus("Master").is_some(), "Should have Master bus");
    assert!(engine.get_bus("Music").is_some(), "Should have Music bus");
    assert!(engine.get_bus("SFX").is_some(), "Should have SFX bus");
    assert!(engine.get_bus("Voice").is_some(), "Should have Voice bus");
    assert!(engine.get_bus("Ambient").is_some(), "Should have Ambient bus");
    
    println!("✅ Default buses created");
}

// ============================================================================
// AudioBus Tests
// ============================================================================

#[test]
fn test_create_custom_bus() {
    let mut engine = AudioEngine::new();
    engine.create_bus("UI".to_string(), Some("Master".to_string()));
    
    let bus = engine.get_bus("UI");
    assert!(bus.is_some());
    assert_eq!(bus.unwrap().name, "UI");
    assert_eq!(bus.unwrap().parent, Some("Master".to_string()));
    
    println!("✅ Custom bus created");
}

#[test]
fn test_bus_volume() {
    let mut engine = AudioEngine::new();
    
    engine.set_bus_volume("Music", 0.5);
    let bus = engine.get_bus("Music").unwrap();
    assert_eq!(bus.volume, 0.5);
    
    engine.set_bus_volume("Music", 1.5); // Should clamp to 1.0
    let bus = engine.get_bus("Music").unwrap();
    assert_eq!(bus.volume, 1.0);
    
    println!("✅ Bus volume works with clamping");
}

#[test]
fn test_bus_mute() {
    let mut engine = AudioEngine::new();
    
    engine.set_bus_muted("SFX", true);
    let bus = engine.get_bus("SFX").unwrap();
    assert!(bus.muted);
    
    engine.set_bus_muted("SFX", false);
    let bus = engine.get_bus("SFX").unwrap();
    assert!(!bus.muted);
    
    println!("✅ Bus mute/unmute works");
}

#[test]
fn test_bus_effects() {
    let mut engine = AudioEngine::new();
    
    let reverb = AudioEffect::Reverb {
        room_size: 0.5,
        damping: 0.5,
        wet: 0.3,
        dry: 0.7,
    };
    
    engine.add_bus_effect("Music", reverb);
    
    let bus = engine.get_bus("Music").unwrap();
    assert_eq!(bus.effects.len(), 1);
    
    println!("✅ Bus effects work");
}

#[test]
fn test_bus_hierarchy() {
    let mut engine = AudioEngine::new();
    
    // Music bus should have Master as parent
    let music_bus = engine.get_bus("Music").unwrap();
    assert_eq!(music_bus.parent, Some("Master".to_string()));
    
    println!("✅ Bus hierarchy works");
}

// ============================================================================
// AudioSource Tests (3D)
// ============================================================================

#[test]
fn test_play_sound_3d() {
    let mut engine = AudioEngine::new();
    
    let id = engine.play_sound_at(
        "explosion.wav".to_string(),
        Vec3::new(10.0, 0.0, 0.0),
        "SFX".to_string(),
    );
    
    assert_eq!(id, 0);
    assert_eq!(engine.source_count(), 1);
    
    let source = &engine.sources[id];
    assert_eq!(source.position, Vec3::new(10.0, 0.0, 0.0));
    assert_eq!(source.spatial_blend, 1.0); // Full 3D
    assert!(source.playing);
    
    println!("✅ 3D sound played");
}

#[test]
fn test_play_sound_2d() {
    let mut engine = AudioEngine::new();
    
    let id = engine.play_sound_2d("ui_click.wav".to_string(), "SFX".to_string());
    
    let source = &engine.sources[id];
    assert_eq!(source.spatial_blend, 0.0); // Full 2D
    assert!(source.playing);
    
    println!("✅ 2D sound played");
}

#[test]
fn test_multiple_sounds() {
    let mut engine = AudioEngine::new();
    
    let id1 = engine.play_sound_2d("sound1.wav".to_string(), "SFX".to_string());
    let id2 = engine.play_sound_2d("sound2.wav".to_string(), "SFX".to_string());
    let id3 = engine.play_sound_2d("sound3.wav".to_string(), "Music".to_string());
    
    assert_eq!(id1, 0);
    assert_eq!(id2, 1);
    assert_eq!(id3, 2);
    assert_eq!(engine.source_count(), 3);
    
    println!("✅ Multiple sounds work");
}

#[test]
fn test_stop_sound() {
    let mut engine = AudioEngine::new();
    
    let id = engine.play_sound_2d("sound.wav".to_string(), "SFX".to_string());
    assert!(engine.sources[id].playing);
    
    engine.stop_sound(id);
    assert!(!engine.sources[id].playing);
    
    println!("✅ Stop sound works");
}

#[test]
fn test_pause_resume_sound() {
    let mut engine = AudioEngine::new();
    
    let id = engine.play_sound_2d("sound.wav".to_string(), "SFX".to_string());
    assert!(!engine.sources[id].paused);
    
    engine.pause_sound(id);
    assert!(engine.sources[id].paused);
    
    engine.resume_sound(id);
    assert!(!engine.sources[id].paused);
    
    println!("✅ Pause/resume works");
}

// ============================================================================
// Spatial Audio Tests
// ============================================================================

#[test]
fn test_listener_position() {
    let mut engine = AudioEngine::new();
    
    engine.set_listener_position(Vec3::new(5.0, 0.0, 0.0));
    assert_eq!(engine.listener_position, Vec3::new(5.0, 0.0, 0.0));
    
    println!("✅ Listener position set");
}

#[test]
fn test_listener_orientation() {
    let mut engine = AudioEngine::new();
    
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::Y;
    
    engine.set_listener_orientation(forward, up);
    assert_eq!(engine.listener_forward, forward.normalize());
    assert_eq!(engine.listener_up, up.normalize());
    
    println!("✅ Listener orientation set");
}

#[test]
fn test_distance_attenuation_linear() {
    let engine = AudioEngine::new();
    
    // At min distance (full volume)
    let vol1 = engine.calculate_distance_attenuation(1.0, 1.0, 100.0, RolloffMode::Linear);
    assert_eq!(vol1, 1.0, "Should be full volume at min distance");
    
    // At max distance (no volume)
    let vol2 = engine.calculate_distance_attenuation(100.0, 1.0, 100.0, RolloffMode::Linear);
    assert_eq!(vol2, 0.0, "Should be silent at max distance");
    
    // Halfway (50% volume)
    let vol3 = engine.calculate_distance_attenuation(50.5, 1.0, 100.0, RolloffMode::Linear);
    assert!(vol3 > 0.4 && vol3 < 0.6, "Should be ~50% at halfway: {}", vol3);
    
    println!("✅ Linear distance attenuation works");
}

#[test]
fn test_distance_attenuation_logarithmic() {
    let engine = AudioEngine::new();
    
    // At min distance
    let vol1 = engine.calculate_distance_attenuation(1.0, 1.0, 100.0, RolloffMode::Logarithmic);
    assert_eq!(vol1, 1.0);
    
    // At 2x min distance (should be 0.5 for logarithmic)
    let vol2 = engine.calculate_distance_attenuation(2.0, 1.0, 100.0, RolloffMode::Logarithmic);
    assert_eq!(vol2, 0.5);
    
    // At 10x min distance (should be 0.1)
    let vol3 = engine.calculate_distance_attenuation(10.0, 1.0, 100.0, RolloffMode::Logarithmic);
    assert_eq!(vol3, 0.1);
    
    println!("✅ Logarithmic distance attenuation works");
}

#[test]
fn test_distance_attenuation_custom() {
    let engine = AudioEngine::new();
    
    let vol1 = engine.calculate_distance_attenuation(1.0, 1.0, 100.0, RolloffMode::Custom);
    assert_eq!(vol1, 1.0);
    
    let vol2 = engine.calculate_distance_attenuation(100.0, 1.0, 100.0, RolloffMode::Custom);
    assert_eq!(vol2, 0.0);
    
    // Custom uses quadratic falloff
    let vol3 = engine.calculate_distance_attenuation(50.5, 1.0, 100.0, RolloffMode::Custom);
    assert!(vol3 > 0.0 && vol3 < 1.0);
    
    println!("✅ Custom distance attenuation works");
}

#[test]
fn test_3d_audio_calculation() {
    let mut engine = AudioEngine::new();
    
    // Listener at origin
    engine.set_listener_position(Vec3::ZERO);
    engine.set_listener_orientation(Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
    
    // Sound to the right
    let id = engine.play_sound_at(
        "sound.wav".to_string(),
        Vec3::new(10.0, 0.0, 0.0),
        "SFX".to_string(),
    );
    
    let source = &engine.sources[id];
    let params = engine.calculate_3d_audio(source);
    
    // Should have some volume attenuation due to distance
    assert!(params.volume > 0.0 && params.volume <= 1.0);
    
    // Should be panned to the right
    assert!(params.pan > 0.0, "Sound to the right should have positive pan");
    
    println!("✅ 3D audio calculation works: vol={}, pan={}", params.volume, params.pan);
}

// ============================================================================
// AudioEffect Tests
// ============================================================================

#[test]
fn test_reverb_effect() {
    let reverb = AudioEffect::Reverb {
        room_size: 0.8,
        damping: 0.5,
        wet: 0.3,
        dry: 0.7,
    };
    
    match reverb {
        AudioEffect::Reverb { room_size, .. } => {
            assert_eq!(room_size, 0.8);
        }
        _ => panic!("Expected Reverb effect"),
    }
    
    println!("✅ Reverb effect created");
}

#[test]
fn test_echo_effect() {
    let echo = AudioEffect::Echo {
        delay: 0.5,
        decay: 0.6,
        wet: 0.4,
    };
    
    match echo {
        AudioEffect::Echo { delay, .. } => {
            assert_eq!(delay, 0.5);
        }
        _ => panic!("Expected Echo effect"),
    }
    
    println!("✅ Echo effect created");
}

#[test]
fn test_filter_effects() {
    let lowpass = AudioEffect::LowPass {
        cutoff_frequency: 1000.0,
        resonance: 0.5,
    };
    
    let highpass = AudioEffect::HighPass {
        cutoff_frequency: 500.0,
        resonance: 0.3,
    };
    
    match lowpass {
        AudioEffect::LowPass { cutoff_frequency, .. } => {
            assert_eq!(cutoff_frequency, 1000.0);
        }
        _ => panic!("Expected LowPass effect"),
    }
    
    match highpass {
        AudioEffect::HighPass { cutoff_frequency, .. } => {
            assert_eq!(cutoff_frequency, 500.0);
        }
        _ => panic!("Expected HighPass effect"),
    }
    
    println!("✅ Filter effects created");
}

#[test]
fn test_distortion_effect() {
    let distortion = AudioEffect::Distortion { amount: 0.7 };
    
    match distortion {
        AudioEffect::Distortion { amount } => {
            assert_eq!(amount, 0.7);
        }
        _ => panic!("Expected Distortion effect"),
    }
    
    println!("✅ Distortion effect created");
}

#[test]
fn test_chorus_effect() {
    let chorus = AudioEffect::Chorus {
        rate: 1.5,
        depth: 0.5,
        mix: 0.3,
    };
    
    match chorus {
        AudioEffect::Chorus { rate, .. } => {
            assert_eq!(rate, 1.5);
        }
        _ => panic!("Expected Chorus effect"),
    }
    
    println!("✅ Chorus effect created");
}

// ============================================================================
// RolloffMode Tests
// ============================================================================

#[test]
fn test_rolloff_modes() {
    assert_ne!(RolloffMode::Linear, RolloffMode::Logarithmic);
    assert_ne!(RolloffMode::Logarithmic, RolloffMode::Custom);
    assert_eq!(RolloffMode::Linear, RolloffMode::Linear);
    
    println!("✅ RolloffMode enum works");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_get_active_sources() {
    let mut engine = AudioEngine::new();
    
    let id1 = engine.play_sound_2d("sound1.wav".to_string(), "SFX".to_string());
    let id2 = engine.play_sound_2d("sound2.wav".to_string(), "SFX".to_string());
    let id3 = engine.play_sound_2d("sound3.wav".to_string(), "SFX".to_string());
    
    // Stop one
    engine.stop_sound(id2);
    
    // Pause one
    engine.pause_sound(id3);
    
    let active = engine.get_active_sources();
    assert_eq!(active.len(), 1, "Only one sound should be active (playing and not paused)");
    assert_eq!(active[0].id, id1);
    
    println!("✅ Get active sources works");
}

#[test]
fn test_complex_audio_scene() {
    let mut engine = AudioEngine::new();
    
    // Set listener position
    engine.set_listener_position(Vec3::ZERO);
    
    // Play ambient music
    let _music = engine.play_sound_2d("music.ogg".to_string(), "Music".to_string());
    
    // Play 3D sounds at various positions
    let _explosion = engine.play_sound_at(
        "explosion.wav".to_string(),
        Vec3::new(20.0, 0.0, 0.0),
        "SFX".to_string(),
    );
    
    let _footsteps = engine.play_sound_at(
        "footsteps.wav".to_string(),
        Vec3::new(5.0, 0.0, -5.0),
        "SFX".to_string(),
    );
    
    // Play UI sound
    let _ui_click = engine.play_sound_2d("click.wav".to_string(), "SFX".to_string());
    
    assert_eq!(engine.source_count(), 4);
    
    // Adjust bus volumes
    engine.set_bus_volume("Music", 0.7);
    engine.set_bus_volume("SFX", 0.8);
    
    assert_eq!(engine.get_bus("Music").unwrap().volume, 0.7);
    assert_eq!(engine.get_bus("SFX").unwrap().volume, 0.8);
    
    println!("✅ Complex audio scene works");
}

