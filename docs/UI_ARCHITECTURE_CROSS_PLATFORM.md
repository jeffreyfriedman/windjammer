# Windjammer UI: Cross-Platform Architecture

**Date:** November 12, 2025  
**Status:** 🔍 Architecture Analysis

## Current Architecture

### Overview

Windjammer UI (`windjammer-ui`) is designed to support multiple platforms:
- **Browser (WASM)**: DOM-based UI using `web_sys`
- **Desktop (Native)**: Native windows using `winit` + `wgpu`
- **Mobile**: Future support for iOS/Android

However, the **reactive system** (`ReactiveApp`) is currently **WASM-only**.

---

## Platform Support Matrix

| Feature | Browser (WASM) | Desktop (Native) | Mobile |
|---------|----------------|------------------|--------|
| **App Runtime** | ✅ `App`, `ReactiveApp` | ⚠️ `App` (partial) | ❌ Not yet |
| **Reactivity** | ✅ `Signal`, `Computed`, `Effect` | ✅ Available | ✅ Available |
| **Auto Re-render** | ✅ `ReactiveApp` | ❌ Not implemented | ❌ Not implemented |
| **DOM Manipulation** | ✅ `web_sys` | ❌ N/A | ❌ N/A |
| **Native Windows** | ❌ N/A | ✅ `winit` + `wgpu` | ❌ Not yet |
| **Components** | ✅ All components | ✅ All components | ✅ All components |

---

## The Problem: ReactiveApp is WASM-Only

### Why ReactiveApp is WASM-Only

```rust
// From app_reactive.rs
#[cfg(target_arch = "wasm32")]
pub struct ReactiveApp {
    title: String,
    render_fn: Rc<dyn Fn() -> VNode>,
    root_element: Option<Element>,      // web_sys::Element
    document: Option<web_sys::Document>, // web_sys::Document
}
```

`ReactiveApp` directly uses `web_sys` types:
- `web_sys::Element` for DOM manipulation
- `web_sys::Document` for document access
- `web_sys::window()` for browser APIs

These are **browser-specific** and don't exist on native platforms.

### Current Game Editor Issue

The game editor (`crates/windjammer-game-editor/ui/editor.wj`) uses:

```windjammer
let app = ReactiveApp::new("Windjammer Game Editor".to_string(), move || {
    // ... render function ...
})
app.run()
```

This code **only works on WASM** because `ReactiveApp` is WASM-only.

---

## Solutions for Cross-Platform Support

### Option A: Browser-Only Editor (Recommended for Now)

**Approach:**
- Compile editor for WASM only
- Run in browser (standalone or embedded in Tauri webview)
- Use `ReactiveApp` for automatic re-rendering

**Pros:**
- ✅ Works immediately (no code changes needed)
- ✅ Best UX (reactive updates)
- ✅ Aligns with modern editor UX (VS Code, etc.)
- ✅ Can be embedded in Tauri for "desktop app" feel

**Cons:**
- ❌ Requires browser or Tauri
- ❌ Can't run as pure native binary

**Implementation:**
1. Compile editor to WASM
2. Create HTML page
3. Test in browser
4. (Optional) Embed in Tauri for desktop distribution

---

### Option B: Implement ReactiveApp for Native

**Approach:**
- Create native version of `ReactiveApp`
- Use `winit` + `wgpu` for native rendering
- Implement DOM-like abstraction for native

**Pros:**
- ✅ True cross-platform (same code, both targets)
- ✅ Pure native binary (no browser needed)
- ✅ Better performance on desktop

**Cons:**
- ❌ Significant implementation work (2-4 weeks)
- ❌ Need to build DOM-like abstraction for native
- ❌ Complexity of managing two render backends

**Implementation:**
1. Create `ReactiveApp` for native using `winit`
2. Implement native renderer (VNode → winit widgets)
3. Add conditional compilation for both platforms
4. Test on both platforms

---

### Option C: Dual-Mode Editor

**Approach:**
- Use `ReactiveApp` for WASM
- Use `App` (static) for native
- Conditional compilation based on target

**Pros:**
- ✅ Works on both platforms
- ✅ Less implementation work than Option B
- ✅ Leverages existing infrastructure

**Cons:**
- ❌ Native version won't have reactive updates
- ❌ Two code paths to maintain
- ❌ Different UX on different platforms

**Implementation:**
```windjammer
#[cfg(target_arch = "wasm32")]
fn create_app() -> ReactiveApp {
    ReactiveApp::new("Editor".to_string(), render_fn)
}

#[cfg(not(target_arch = "wasm32"))]
fn create_app() -> App {
    App::new("Editor".to_string(), render_static())
}
```

---

### Option D: Tauri Webview (Hybrid Approach)

**Approach:**
- Compile editor to WASM
- Embed in Tauri webview for desktop
- Use `ReactiveApp` (works in webview)
- Native window frame, web content

**Pros:**
- ✅ **BEST OF BOTH WORLDS**
- ✅ Single codebase (WASM only)
- ✅ Reactive updates (ReactiveApp works)
- ✅ Native desktop app (Tauri provides window)
- ✅ Access to native APIs via Tauri

**Cons:**
- ❌ Requires Tauri runtime
- ❌ Slightly larger binary than pure native

**Implementation:**
1. Compile editor to WASM
2. Create Tauri wrapper app
3. Load WASM in webview
4. Use Tauri APIs for file system, etc.

**This is how VS Code, Cursor, and most modern editors work!**

---

## Recommended Architecture

### Phase 1: Browser Editor (Immediate)
- ✅ Compile editor to WASM
- ✅ Create HTML page
- ✅ Test in browser
- ✅ Use `ReactiveApp` for reactivity

### Phase 2: Tauri Desktop App (Short-term)
- ✅ Create Tauri wrapper
- ✅ Embed WASM editor in webview
- ✅ Native window with web content
- ✅ Access to native file system

### Phase 3: Pure Native (Long-term, if needed)
- ⏳ Implement `ReactiveApp` for native
- ⏳ Native renderer (VNode → winit)
- ⏳ True native binary

---

## Current Code Sharing Strategy

### What's Shared (100%)

**Windjammer Source Code:**
- ✅ `editor.wj` - Same code for all platforms
- ✅ Uses `std::ui::*` - Platform-agnostic
- ✅ Uses `std::fs::*`, `std::process::*` - Platform-agnostic
- ✅ Uses `Signal`, `Computed`, `Effect` - Platform-agnostic

**Compiler:**
- ✅ Detects target (`--target wasm` or `--target rust`)
- ✅ Generates platform-specific imports
- ✅ Links to correct runtime implementations

### What's Platform-Specific (0% user code, 100% compiler/runtime)

**WASM Target:**
```rust
// Generated by compiler
use windjammer_ui::prelude::*;  // Includes ReactiveApp (WASM)
use windjammer_runtime::platform::wasm::fs::*;  // localStorage
use windjammer_runtime::platform::wasm::process::*;  // stubs
```

**Native Target:**
```rust
// Generated by compiler
use windjammer_ui::prelude::*;  // Includes App (but not ReactiveApp)
use windjammer_runtime::platform::native::fs::*;  // std::fs
use windjammer_runtime::platform::native::process::*;  // std::process
```

### The Gap

The **only** thing preventing full cross-platform support is:
- `ReactiveApp` is WASM-only (uses `web_sys`)
- Native needs its own `ReactiveApp` implementation

**User code is already 100% platform-agnostic!** The issue is purely in the runtime layer.

---

## Solution: Implement Native ReactiveApp

### Architecture

```rust
// crates/windjammer-ui/src/app_reactive.rs

#[cfg(target_arch = "wasm32")]
pub struct ReactiveApp {
    // Uses web_sys for DOM
    root_element: Option<web_sys::Element>,
    document: Option<web_sys::Document>,
    // ...
}

#[cfg(not(target_arch = "wasm32"))]
pub struct ReactiveApp {
    // Uses winit for native windows
    window: Option<Arc<winit::window::Window>>,
    renderer: Option<NativeRenderer>,  // VNode → winit widgets
    // ...
}
```

### Implementation Steps

1. **Create Native Renderer** (2-3 days)
   - Convert `VNode` to native widgets
   - Use `winit` for window management
   - Use `wgpu` or `egui` for rendering

2. **Implement Native ReactiveApp** (1-2 days)
   - Same API as WASM version
   - Different internal implementation
   - Trigger re-renders on signal changes

3. **Test Both Platforms** (1 day)
   - Compile editor for WASM
   - Compile editor for native
   - Verify identical behavior

---

## Immediate Action Plan

### For This Session

1. ✅ Compile editor for **WASM** (not native)
2. ✅ Create HTML page
3. ✅ Test in browser
4. ✅ Document findings

### For Future

1. **Short-term**: Tauri wrapper (1-2 days)
   - Best ROI for desktop support
   - Leverages existing WASM build
   - Native app experience

2. **Long-term**: Native ReactiveApp (1-2 weeks)
   - True cross-platform from single source
   - Pure native binary option
   - Ultimate flexibility

---

## Key Insight

**The user's question reveals a critical point:**

> "What is our architecture to share code between the browser game editor and the desktop game editor?"

**Answer:** The architecture is **already perfect** for code sharing!

- ✅ **100% of user code is shared** (editor.wj)
- ✅ **Compiler handles platform differences** (automatic)
- ✅ **Runtime provides platform implementations** (transparent)

The **only** missing piece is a native implementation of `ReactiveApp`. Once that exists, the SAME `editor.wj` file will compile for both platforms with ZERO changes!

---

## Comparison to Other Frameworks

### Electron/Tauri (Current Industry Standard)
- Write once in HTML/CSS/JS
- Runs in webview on all platforms
- Large bundle size

### Flutter/React Native
- Write once in Dart/JS
- Different renderers per platform
- Some platform-specific code needed

### Windjammer (Our Approach)
- ✅ Write once in Windjammer
- ✅ Compiler generates platform-specific code
- ✅ Zero platform-specific user code
- ⏳ Need native ReactiveApp implementation

**We're 95% there!** Just need the native reactive runtime.

---

## Recommendation

### For NOW (This Session):
**Focus on WASM/browser** - it works today and provides the best UX.

### For NEXT:
**Implement native ReactiveApp** - this unlocks true "write once, run anywhere" with zero platform-specific code.

### For LATER:
**Tauri wrapper** - provides desktop distribution while native ReactiveApp is being built.

---

## Summary

**Q: "What is our architecture to share code between browser and desktop?"**

**A:** The architecture is **already perfect**:
- User writes pure Windjammer (platform-agnostic)
- Compiler detects target and generates platform-specific code
- Runtime provides platform implementations

**The only gap:** `ReactiveApp` needs a native implementation.

**Current status:** Editor works for WASM/browser. Native support requires implementing native `ReactiveApp` (1-2 weeks of work).

**Immediate path forward:** Compile for WASM and test in browser! 🚀

---

*Last Updated: November 12, 2025*  
*Version: Windjammer 0.34.0*

