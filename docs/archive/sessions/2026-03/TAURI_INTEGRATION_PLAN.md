# Tauri Integration Plan - Pure Windjammer Approach

**Date**: November 13, 2025  
**Status**: In Progress

## Problem

The Windjammer editor is written in pure Windjammer (`editor.wj`) and compiles to WASM. Currently:
- ✅ Browser: Uses localStorage (works)
- ❌ Tauri Desktop: Still uses localStorage (should use real FS)

The buttons work, but they're calling localStorage instead of Tauri commands.

## Core Principle

**Developers write ONLY Windjammer code. No JavaScript. No Tauri code.**

```windjammer
// editor.wj - Pure Windjammer
use std::fs::*
use std::process::*

// This should "just work" in both browser and desktop
fs::write_file("game.wj", code)
process::execute("wj", ["build", "game.wj"])
```

## Solution Options

### Option A: Async/Await in Windjammer (RECOMMENDED)
Make `std::fs` and `std::process` async in Windjammer:

```windjammer
// editor.wj
async fn save_file() {
    await fs::write_file(path, content)  // Works in browser AND Tauri
}
```

**Pros:**
- Clean, modern API
- Works everywhere (browser, Tauri, native)
- No platform-specific code

**Cons:**
- Requires adding async/await to Windjammer language
- Breaking change to stdlib

### Option B: Synchronous Wrapper with Hidden Async
Keep synchronous API, but use hidden async internally:

```rust
// In windjammer-runtime/platform/wasm/fs.rs
pub fn write_file(path: String, content: String) -> FsResult<()> {
    if is_tauri() {
        // Use wasm-bindgen-futures to block on Tauri call
        block_on(tauri_write_file(path, content))
    } else {
        // Use localStorage (synchronous)
        localstorage_write_file(path, content)
    }
}
```

**Pros:**
- No language changes
- Works with existing editor code
- Transparent to developers

**Cons:**
- Blocking async is not ideal
- May cause UI freezes for large operations

### Option C: Native Rust for Tauri (CURRENT APPROACH)
Use WASM for browser, native Rust for Tauri:

```
Browser:  editor.wj → WASM → localStorage
Desktop:  editor.wj → Native Rust → Real FS (no Tauri needed!)
```

**Pros:**
- Best performance (no WASM overhead in desktop)
- Uses real FS directly
- No async issues

**Cons:**
- Can't use DOM/webview for UI in native
- Need egui or similar for native UI
- Two different rendering paths

### Option D: Tauri with Native Backend Commands
Keep WASM frontend, but all FS/process operations go through Tauri:

```
editor.wj → WASM (UI only) → Tauri IPC → Native Rust (FS/process)
```

**Pros:**
- Reuses WASM UI
- Real FS/process operations
- Works today with Tauri

**Cons:**
- Requires Tauri-specific runtime code
- IPC overhead
- Async/sync mismatch

## Recommended Approach

**Phase 1: Make it work (Option D)**
1. Keep WASM editor as-is
2. Add Tauri bridge that intercepts FS/process calls
3. Make bridge transparent (auto-detected)
4. Developers write pure Windjammer, runtime handles Tauri

**Phase 2: Make it better (Option A)**
1. Add async/await to Windjammer language
2. Make `std::fs` and `std::process` async
3. Update editor to use async
4. Now works perfectly everywhere

## Implementation (Phase 1)

### 1. Update WASM Runtime to Auto-Detect Tauri

```rust
// windjammer-runtime/src/platform/wasm/fs.rs

pub fn write_file(path: String, content: String) -> FsResult<()> {
    if is_tauri_available() {
        // Call Tauri command via JS interop
        call_tauri_write_file(&path, &content)
    } else {
        // Use localStorage
        localstorage_write_file(path, content)
    }
}

fn is_tauri_available() -> bool {
    // Check if window.__TAURI__ exists
    js_sys::Reflect::has(&window(), &JsValue::from_str("__TAURI__"))
        .unwrap_or(false)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    fn invoke(cmd: &str, args: JsValue) -> Promise;
}
```

### 2. No Changes Needed to editor.wj

The editor code stays pure Windjammer:

```windjammer
// editor.wj - NO CHANGES NEEDED
use std::fs::*

Button::new("Save").on_click(move || {
    fs::write_file(path, content)  // Automatically uses Tauri when available!
})
```

### 3. Tauri App Configuration

```json
// tauri.conf.json
{
  "build": {
    "frontendDist": "../dist"  // Contains WASM files
  },
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'"
    }
  }
}
```

### 4. Tauri Commands (Backend)

```rust
// src-tauri/src/main.rs
#[tauri::command]
async fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content)
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            write_file,
            read_file,
            // ... other commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Developer Experience

### What Developers Write (Pure Windjammer)

```windjammer
// game_editor.wj
use std::ui::*
use std::fs::*
use std::process::*

fn start() {
    let code = Signal::new("".to_string())
    
    ReactiveApp::new("Editor", move || {
        Container::new()
            .child(Button::new("Save").on_click(move || {
                fs::write_file("game.wj", code.get())
            }))
            .child(Button::new("Run").on_click(move || {
                process::execute("wj", vec!["run", "game.wj"])
            }))
    }).run()
}
```

### What Happens Automatically

**In Browser:**
```
fs::write_file() → localStorage.setItem()
process::execute() → Error("Not supported in browser")
```

**In Tauri Desktop:**
```
fs::write_file() → Tauri IPC → std::fs::write()
process::execute() → Tauri IPC → std::process::Command
```

**In Native Desktop (future):**
```
fs::write_file() → std::fs::write() (direct)
process::execute() → std::process::Command (direct)
```

## Zero JavaScript/Tauri Knowledge Required

Developers never see:
- ❌ JavaScript code
- ❌ Tauri `invoke()` calls
- ❌ IPC setup
- ❌ Platform detection

They only write:
- ✅ Pure Windjammer
- ✅ Standard library APIs
- ✅ Platform-agnostic code

## Next Steps

1. ✅ Implement `is_tauri_available()` in WASM runtime
2. ✅ Add Tauri command wrappers to WASM FS module
3. ✅ Add Tauri command wrappers to WASM process module
4. ✅ Test with existing editor.wj (no changes)
5. ✅ Document the transparent behavior

## Future: Async/Await

Once Windjammer has async/await:

```windjammer
// Future editor.wj with async
async fn save_file(path: string, content: string) {
    await fs::write_file(path, content)
}

Button::new("Save").on_click(async move || {
    await save_file(current_file.get(), code.get())
    console.set("✓ Saved!")
})
```

This will work perfectly in all environments without any runtime hacks.

---

**Conclusion**: Developers write pure Windjammer. The runtime automatically detects and uses Tauri when available. Zero platform-specific code required.

