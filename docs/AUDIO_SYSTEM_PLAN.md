# 🔊 Audio System Implementation Plan

**Goal:** Add comprehensive audio support to exercise Windjammer's capabilities

---

## 📋 **Requirements**

### Core Features
1. **Audio Loading** - Load WAV/OGG/MP3 from disk
2. **Sound Effects** - Play one-shot sounds (gunshots, hits, pickups)
3. **Background Music** - Loop music tracks
4. **Volume Control** - Master, SFX, and music volumes
5. **3D Spatial Audio** - Position-based audio (optional)

### Windjammer API Design
```windjammer
// Simple, zero-crate-leakage API
struct Sound {
    id: int,
}

// Load sound from file
fn load_sound(path: string) -> Sound

// Play sound
fn play_sound(sound: Sound)
fn play_sound_at(sound: Sound, volume: float)

// Background music
fn play_music(path: string)
fn stop_music()
fn set_music_volume(volume: float)
```

---

## 🏗️ **Implementation Steps**

### Phase 1: Rust Backend (Framework)
1. Add `rodio` crate (already in Cargo.toml!)
2. Create `AudioSystem` struct
3. Implement sound loading
4. Implement playback
5. Add volume control

### Phase 2: Windjammer API
1. Add `Sound` type to Windjammer
2. Add audio functions
3. Update codegen for audio

### Phase 3: Game Integration
1. Load sound effects in `@init`
2. Play sounds on events (shoot, hit, pickup)
3. Add background music
4. Volume controls

---

## 🎯 **Language Gaps to Surface**

### 1. **Resource Management**
- How does Windjammer handle audio handles?
- How do we manage audio lifetime?
- Thread safety for audio playback?

### 2. **Concurrency**
- Audio plays on separate thread
- How does Windjammer handle async audio?
- Message passing for audio commands?

### 3. **State Management**
- Global audio state
- Per-sound state
- Music playback state

---

## 📝 **Implementation**

### Step 1: Audio System Struct
```rust
// crates/windjammer-game-framework/src/audio.rs
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

pub struct AudioSystem {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    music_sink: Arc<Mutex<Option<Sink>>>,
    sfx_volume: Arc<Mutex<f32>>,
    music_volume: Arc<Mutex<f32>>,
}

impl AudioSystem {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        
        Ok(Self {
            _stream: stream,
            stream_handle,
            music_sink: Arc::new(Mutex::new(None)),
            sfx_volume: Arc::new(Mutex::new(1.0)),
            music_volume: Arc::new(Mutex::new(0.5)),
        })
    }
    
    pub fn play_sound(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let volume = *self.sfx_volume.lock().unwrap();
        self.stream_handle.play_raw(source.amplify(volume).convert_samples())?;
        Ok(())
    }
    
    pub fn play_music(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?.repeat_infinite();
        let sink = Sink::try_new(&self.stream_handle)?;
        let volume = *self.music_volume.lock().unwrap();
        sink.set_volume(volume);
        sink.append(source);
        *self.music_sink.lock().unwrap() = Some(sink);
        Ok(())
    }
    
    pub fn stop_music(&self) {
        if let Some(sink) = self.music_sink.lock().unwrap().take() {
            sink.stop();
        }
    }
    
    pub fn set_sfx_volume(&self, volume: f32) {
        *self.sfx_volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }
    
    pub fn set_music_volume(&self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        *self.music_volume.lock().unwrap() = volume;
        if let Some(sink) = self.music_sink.lock().unwrap().as_ref() {
            sink.set_volume(volume);
        }
    }
}
```

### Step 2: Windjammer API
```windjammer
// In game code
@init
fn init(game: ShooterGame, audio: AudioSystem) {
    // Play background music
    audio.play_music("assets/music/theme.ogg")
    
    // Set volumes
    audio.set_music_volume(0.5)
    audio.set_sfx_volume(0.8)
}

@input
fn handle_input(game: ShooterGame, input: Input, audio: AudioSystem) {
    if input.mouse_pressed(MouseButton::Left) {
        game.shoot()
        audio.play_sound("assets/sfx/shoot.wav")
    }
}
```

### Step 3: Procedural Audio
For testing without files, generate simple tones:
```rust
pub fn generate_beep(frequency: f32, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let samples = (sample_rate * duration) as usize;
    let mut data = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin();
        data.push(sample * 0.3); // 30% volume
    }
    
    data
}
```

---

## 🧪 **Testing Strategy**

### Unit Tests
- Load valid audio file
- Handle missing file
- Handle invalid format
- Volume clamping

### Integration Tests
- Play sound effect
- Play background music
- Stop music
- Volume control

### Game Tests
- Shoot sound plays
- Hit sound plays
- Pickup sound plays
- Music loops correctly

---

## 🚀 **Next Steps**

1. Implement AudioSystem in framework
2. Add Windjammer API
3. Generate procedural sounds for testing
4. Integrate into shooter game
5. Test and document

---

**Status:** Ready to implement!

**Complexity:** Medium (audio is well-supported by rodio)

**Exercise Value:** High (concurrency, resource management, state)

