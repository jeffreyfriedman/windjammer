# Editor Architecture Decision

## Decision: Use Pure egui (No Tauri)

### Context

The current editor (`crates/windjammer-game-editor/src/main.rs`) uses Tauri, which requires JavaScript and a web view. Based on previous discussions and the Windjammer philosophy, we should use pure egui instead.

## Architecture Comparison

### Option 1: Tauri (Current)
**Pros:**
- ✅ Cross-platform
- ✅ Web technologies familiar to many
- ✅ Rich ecosystem

**Cons:**
- ❌ Requires JavaScript
- ❌ Web view overhead
- ❌ Two languages (Rust + JS)
- ❌ Heavier runtime
- ❌ More complex debugging
- ❌ Against Windjammer philosophy

### Option 2: Pure egui (Recommended)
**Pros:**
- ✅ Pure Rust (no JavaScript)
- ✅ Native performance
- ✅ Simpler architecture
- ✅ Easier debugging
- ✅ Code sharing with WASM
- ✅ Mobile support (via egui)
- ✅ Follows Windjammer philosophy
- ✅ Same UI code for desktop & browser

**Cons:**
- ⚠️ Need to implement file dialogs (easy with `rfd` crate)
- ⚠️ Need to implement window management (easy with `winit`)

### Option 3: egui + WASM (Browser)
**Pros:**
- ✅ Same code as desktop
- ✅ Runs in browser
- ✅ No installation needed
- ✅ Cross-platform

**Implementation:**
- Use `eframe` for desktop
- Use `eframe` with WASM target for browser
- Share 99% of code between platforms

## Decision

**Use pure egui for both desktop and browser.**

### Desktop Editor
```rust
use eframe::egui;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Windjammer Game Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "Windjammer Editor",
        options,
        Box::new(|cc| Box::new(EditorApp::new(cc))),
    ).unwrap();
}
```

### Browser Editor (WASM)
```rust
#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "canvas",
                web_options,
                Box::new(|cc| Box::new(EditorApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
```

## Implementation Plan

### Phase 1: Replace Tauri with egui (Desktop)
1. Remove Tauri dependencies
2. Add `eframe`, `egui`, `rfd` (file dialogs)
3. Rewrite editor using pure egui
4. Implement file I/O with native Rust
5. Add window management

### Phase 2: Add WASM Support (Browser)
1. Add WASM build target
2. Implement IndexedDB storage
3. Add Web Workers for compilation
4. Build and deploy

### Phase 3: Shared Features
Both desktop and browser share:
- UI code (egui)
- Editor logic
- Project management
- Scene editing
- Asset pipeline

Platform-specific:
- Desktop: Native file I/O
- Browser: IndexedDB + upload/download

## Benefits of This Approach

### 1. Pure Windjammer Philosophy
- One language (Rust)
- One UI framework (egui)
- No abstraction leakage
- Simple and elegant

### 2. Code Sharing
- 99% code shared between desktop & browser
- Same UI, same logic
- Platform differences isolated

### 3. Performance
- Native rendering (no web view)
- Direct GPU access
- Minimal overhead

### 4. Mobile Support
- egui works on iOS/Android
- Same code, different target
- Future-proof

### 5. Development Experience
- Single language debugging
- Faster iteration
- Simpler architecture

## Migration Path

### Step 1: Create New egui Editor
Location: `crates/windjammer-game-editor/src/editor_egui.rs`

### Step 2: Port Existing Features
- Project management
- File browser
- Code editor
- Scene view
- Property inspector
- Console

### Step 3: Remove Tauri
- Delete Tauri config
- Remove JS dependencies
- Clean up old code

### Step 4: Add WASM Target
- Configure for WASM
- Add web-specific code
- Deploy

## File Structure

```
crates/windjammer-game-editor/
├── Cargo.toml              # egui dependencies
├── src/
│   ├── main.rs            # Desktop entry (egui)
│   ├── lib.rs             # Shared editor code
│   ├── app.rs             # Main editor app
│   ├── panels/            # UI panels
│   │   ├── hierarchy.rs
│   │   ├── inspector.rs
│   │   ├── assets.rs
│   │   ├── console.rs
│   │   └── viewport.rs
│   ├── tools/             # Editing tools
│   └── platform/          # Platform-specific
│       ├── desktop.rs     # Native file I/O
│       └── web.rs         # IndexedDB, etc.
└── web/                   # WASM build output
```

## Dependencies

### Desktop
```toml
[dependencies]
eframe = "0.24"
egui = "0.24"
rfd = "0.12"  # File dialogs
winit = "0.29"  # Window management
```

### Browser (additional)
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = "0.3"
js-sys = "0.3"
```

## Timeline

- **Phase 1 (Desktop)**: 1-2 weeks
- **Phase 2 (Browser)**: 1 week
- **Phase 3 (Polish)**: 1 week

**Total**: 3-4 weeks for complete implementation

## Conclusion

**Use pure egui for both desktop and browser editions.**

This aligns with:
- ✅ Windjammer philosophy (pure, simple, elegant)
- ✅ Your previous decision to "go all the way with egui"
- ✅ Mobile support requirement
- ✅ Code sharing goals
- ✅ Performance requirements

**Status**: ✅ **DECISION MADE**  
**Next**: Implement pure egui editor

