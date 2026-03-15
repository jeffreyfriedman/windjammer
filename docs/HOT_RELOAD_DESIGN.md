# Hot Reload System Design

## Overview

**Goal:** Enable instant iteration on shaders and game code without restarting the game.

**Target DX:**
```
Edit shader → See changes in <1 second
Edit game code → See changes in <10 seconds
```

**Competitive with:** Unity, Unreal (both have shader hot reload)

---

## Phase 1: Shader Hot Reload (RECOMMENDED FIRST)

### Architecture

```
┌─────────────────┐
│ File watcher    │
│ (notify crate)  │
└────────┬────────┘
         │ detects: shaders/*.wgsl
         ↓
┌─────────────────┐
│ Reload handler  │
│ (runtime-host)  │
└────────┬────────┘
         │ actions:
         ├─> Recompile WGSL
         ├─> Recreate shader module
         └─> Replace shader_id in map
         ↓
┌─────────────────┐
│ Game continues  │
│ (no restart!)   │
└─────────────────┘
```

### Current State

**Already exists:**
- `api::hot_reload_enable()` (FFI stub)
- `api::hot_reload_poll()` (FFI stub)
- `gpu_load_compute_shader_from_file(path)` (loads WGSL)

**Missing:**
- Path tracking (shader_id → path mapping)
- Reload logic (recompile + swap)
- File watcher integration
- Error overlay

### Implementation Plan

#### 1. Add Path Tracking

**File:** `windjammer-runtime-host/src/gpu_compute.rs`

```rust
// Add to RUNTIME struct
struct RuntimeState {
    // ... existing fields ...
    shader_paths: HashMap<u32, PathBuf>,  // shader_id → path
}

// Modify load function
pub extern "C" fn gpu_load_compute_shader_from_file(path: *const c_char) -> u32 {
    // ... existing code to load shader ...
    
    // Track path
    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap() };
    runtime.shader_paths.insert(shader_id, PathBuf::from(path_str));
    
    shader_id
}
```

#### 2. Add Reload API

```rust
pub extern "C" fn gpu_reload_shader_by_path(path: *const c_char) -> bool {
    let runtime = RUNTIME.lock().unwrap();
    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap() };
    
    // Find shader_id for this path
    let shader_id = match runtime.shader_paths.iter()
        .find(|(_, p)| p.as_path() == Path::new(path_str))
        .map(|(id, _)| *id)
    {
        Some(id) => id,
        None => {
            eprintln!("⚠️  Shader not loaded: {}", path_str);
            return false;
        }
    };
    
    // Read new source
    let source = match fs::read_to_string(path_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to read {}: {}", path_str, e);
            return false;
        }
    };
    
    // Compile new shader
    let shader_module = match runtime.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(path_str),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    }) {
        Ok(module) => module,
        Err(e) => {
            eprintln!("❌ Shader compilation failed: {}", e);
            eprintln!("   Keeping old shader");
            return false;
        }
    };
    
    // Replace in map (atomic swap)
    runtime.shader_modules.insert(shader_id, shader_module);
    
    println!("✅ Hot reloaded: {}", path_str);
    true
}
```

#### 3. Add File Watcher

**Dependency:** `notify = "6.0"`

```rust
// src/hot_reload.rs (new file)
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc::{channel, Receiver};
use std::path::PathBuf;

pub struct HotReloadWatcher {
    _watcher: RecommendedWatcher,  // Keep alive
    rx: Receiver<Event>,
}

impl HotReloadWatcher {
    pub fn new(shader_dir: PathBuf) -> Self {
        let (tx, rx) = channel();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }).unwrap();
        
        watcher.watch(&shader_dir, RecursiveMode::Recursive).unwrap();
        
        Self { _watcher: watcher, rx }
    }
    
    pub fn check_for_changes(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        
        while let Ok(event) = self.rx.try_recv() {
            if matches!(event.kind, EventKind::Modify(_)) {
                for path in event.paths {
                    if path.extension() == Some(OsStr::new("wgsl")) {
                        changed.push(path);
                    }
                }
            }
        }
        
        changed
    }
}
```

#### 4. Integrate with Main Loop

**File:** `windjammer-runtime-host/src/window.rs`

```rust
// Add to window state
struct WindowState {
    // ... existing ...
    hot_reload_watcher: Option<HotReloadWatcher>,
}

// In hot_reload_enable
pub extern "C" fn hot_reload_enable() {
    let mut runtime = RUNTIME.lock().unwrap();
    
    let shader_dir = std::env::current_dir()
        .unwrap()
        .join("shaders");
    
    let watcher = HotReloadWatcher::new(shader_dir);
    runtime.hot_reload_watcher = Some(watcher);
    
    println!("🔥 Hot reload enabled for shaders/");
}

// In hot_reload_poll (called each frame)
pub extern "C" fn hot_reload_poll() {
    let mut runtime = RUNTIME.lock().unwrap();
    
    if let Some(watcher) = &mut runtime.hot_reload_watcher {
        let changed = watcher.check_for_changes();
        
        for path in changed {
            let path_cstr = CString::new(path.to_str().unwrap()).unwrap();
            gpu_reload_shader_by_path(path_cstr.as_ptr());
        }
    }
}
```

#### 5. Enable in Game

**File:** `breach-protocol/src_wj/game.wj`

```windjammer
impl Game {
    pub fn initialize(self) {
        // ... existing initialization ...
        
        // Enable hot reload in debug builds
        if cfg!(debug_assertions) {
            hot_reload_enable()
            println("[game] 🔥 Shader hot reload enabled")
        }
        
        // ... rest ...
    }
    
    pub fn update(self, dt: f32) {
        // Poll for shader changes
        hot_reload_poll()
        
        // ... rest of game loop ...
    }
}
```

---

## Phase 2: Game Code Hot Reload (LATER)

### Architecture

```
┌─────────────────┐
│ File watcher    │
│ (notify crate)  │
└────────┬────────┘
         │ detects: src_wj/*.wj
         ↓
┌─────────────────┐
│ wj build        │
│ (transpile)     │
└────────┬────────┘
         │ generates: src/*.rs
         ↓
┌─────────────────┐
│ Rebuild dylib   │
│ (cargo build)   │
└────────┬────────┘
         │ output: libgame.dylib
         ↓
┌─────────────────┐
│ Reload dylib    │
│ (libloading)    │
└────────┬────────┘
         │ actions:
         ├─> Serialize game state
         ├─> Unload old dylib
         ├─> Load new dylib
         └─> Deserialize state
         ↓
┌─────────────────┐
│ Game continues  │
│ (state preserved)│
└─────────────────┘
```

### Challenges

1. **State preservation:** Need serialize/deserialize for all game state
2. **ABI stability:** FFI must remain compatible across reloads
3. **Build time:** Rust incremental build still takes 5-10s
4. **Complexity:** Much harder than shader reload

### Recommendation

**Phase 1 (shader reload) provides 80% of the value for 20% of the effort.**

Game code changes are less frequent than shader tweaks, so 5-10s rebuild is acceptable for now.

---

## Performance Analysis

| Operation | Phase 1 (Shader) | Phase 2 (Code) |
|-----------|------------------|----------------|
| **Detect change** | <1ms | <1ms |
| **Recompile** | ~50ms (WGSL) | ~5-10s (Rust) |
| **Swap** | <10ms | ~100ms |
| **Total** | **~60ms (<1 frame!)** | **~5-10s** |
| **DX impact** | 🚀 Instant iteration | ⚡ Still faster than restart |

---

## Implementation Roadmap

### Phase 1: Shader Hot Reload (P2)

- [ ] Add path tracking (1 hour)
- [ ] Add reload API (2 hours)
- [ ] Add `notify` watcher (2 hours)
- [ ] Integrate with main loop (1 hour)
- [ ] Add error overlay (2 hours)
- [ ] Test with breach-protocol (1 hour)
- [ ] Documentation (1 hour)

**Total: ~10 hours**

### Phase 2: Game Code Hot Reload (Future)

- [ ] Design state serialization protocol (1 day)
- [ ] Convert game to cdylib (1 day)
- [ ] Implement dylib reload (2 days)
- [ ] State preservation (2 days)
- [ ] Testing (1 day)

**Total: ~1-2 weeks**

---

## Files Created

- `/Users/jeffreyfriedman/src/wj/HOT_RELOAD_DESIGN.md`

**Next steps:** Implement Phase 1 (shader hot reload) with TDD after build is fixed.
