# UI Framework Decision: One Way To Do Things

## The Windjammer Philosophy

Windjammer follows Go's philosophy: **There's one way to do things.**

This keeps the language simple, predictable, and easy to learn.

---

## UI Framework History

### ❌ Old: `windjammer-runtime/ui/` (DELETED)
- **Created**: Early proof-of-concept
- **Purpose**: Demonstrate UI generation
- **Status**: Superseded and removed
- **Why removed**: Redundant, confusing, violated "one way" philosophy

### ✅ Current: `windjammer-ui` crate (OFFICIAL)
- **Created**: Production-ready replacement
- **Purpose**: Full-featured reactive UI framework
- **Status**: Active, maintained, official
- **Supports**: Both WASM and native targets

---

## Decision: Delete Old UI Module

**Date**: November 11, 2025

**Reason**: 
- `windjammer-ui` does everything the old module did, and more
- Having two UI systems violates Windjammer's "one way" philosophy
- Causes confusion for users
- No backward compatibility concerns (WASM is new)

**Action**: Removed `crates/windjammer-runtime/src/ui/` entirely

---

## The One True UI Framework

```windjammer
use std::ui::*

// This uses windjammer-ui crate
// Works on WASM, native, everywhere
let app = ReactiveApp::new("My App", || {
    Container::new()
        .child(Text::new("Hello!"))
        .to_vnode()
})

app.run()
```

**Compiler generates**:
```rust
use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
// ONE framework, works everywhere
```

---

## Benefits of One Framework

### 1. **Simplicity**
- Users learn one API
- No confusion about which to use
- Clear documentation

### 2. **Consistency**
- Same code works everywhere
- Same behavior across platforms
- Predictable results

### 3. **Maintainability**
- One codebase to maintain
- All improvements benefit everyone
- No duplicate effort

### 4. **Go-like Philosophy**
- "There should be one-- and preferably only one --obvious way to do it"
- Reduces cognitive load
- Faster development

---

## Platform Support

### WASM (Browser)
```rust
#[cfg(target_arch = "wasm32")]
// windjammer-ui uses web-sys
// Renders to DOM
```

### Native (Desktop)
```rust
#[cfg(not(target_arch = "wasm32"))]
// windjammer-ui can use:
// - Native rendering (future)
// - Tauri webview (current)
// - GTK/Qt bindings (future)
```

**Same API, different backends!**

---

## Migration Guide

If you were using the old `windjammer-runtime/ui/`:

### Before (OLD - REMOVED)
```rust
use windjammer_runtime::ui::{VNode, VElement};
// This no longer exists!
```

### After (NEW - OFFICIAL)
```windjammer
use std::ui::*

// Use the official framework
let ui = Container::new()
    .child(Text::new("Hello!"))
    .to_vnode()
```

---

## Future Direction

### Short Term
- ✅ One UI framework (`windjammer-ui`)
- ✅ WASM support complete
- ⏳ Native rendering improvements

### Long Term
- 🎯 Native GPU rendering
- 🎯 Mobile support (iOS/Android)
- 🎯 Embedded UI (microcontrollers)

**All using the same `windjammer-ui` API!**

---

## Summary

- **One framework**: `windjammer-ui`
- **One API**: `std::ui::*`
- **One way**: Windjammer philosophy
- **Everywhere**: WASM, native, future platforms

**Simplicity wins.** ✅

