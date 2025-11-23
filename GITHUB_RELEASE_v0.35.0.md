# Windjammer v0.35.0 - Clean Stdlib Separation

**Release Date:** November 23, 2025  
**Type:** Major Release (Breaking Changes)

---

## 🎯 Overview

This release achieves **clean separation of concerns** by removing all UI and game code from the Windjammer compiler stdlib. The compiler now focuses purely on language features, while UI and game functionality live in their respective separate crates.

---

## 💥 Breaking Changes

### **Removed `std::ui` Module**

The UI stdlib has been completely removed from the compiler:

**What was removed:**
- `std/ui/` directory and all UI stdlib code
- `std/ui.wj` file
- `windjammer-runtime/src/ui.rs` runtime implementation

**Migration path:**
```toml
# Add to your Cargo.toml:
[dependencies]
windjammer-ui = { git = "https://github.com/jeffreyfriedman/windjammer-ui" }
# Or once published:
# windjammer-ui = "0.1.0"
```

```windjammer
// Your Windjammer code stays the same:
use std::ui::*

fn main() {
    let button = Button::new("Click Me")
    // ... UI code works as before
}
```

**Why this is better:**
- ✅ No circular dependencies
- ✅ Compiler focuses on language features only
- ✅ UI framework can evolve independently
- ✅ Explicit dependency management

### **Removed `std::game` Module**

The game stdlib has been completely removed from the compiler:

**What was removed:**
- `std/game/` directory and all game stdlib code
- `std/game.wj` file  
- `windjammer-runtime/src/game.rs` runtime implementation
- `windjammer-runtime/src/game/` directory (ECS, physics, rendering)

**Migration path:**
```toml
# Add to your Cargo.toml:
[dependencies]
game-host = { git = "https://github.com/jeffreyfriedman/game-host" }
```

**Why this is better:**
- ✅ Clean separation of language vs game engine
- ✅ Game framework can be updated independently
- ✅ Smaller compiler binary
- ✅ Users only pay for what they use

---

## 🔧 Fixed

### **Cargo Publish Dependency Requirements**
- ✅ Added `version = "0.35.0"` to `windjammer` dependency in `windjammer-lsp`
- ✅ All workspace crates now properly specify version requirements
- ✅ Ready for publishing to crates.io without errors

This fixes the publish failure from v0.34.3:
```
error: all dependencies must have a version requirement specified when publishing.
dependency `windjammer` does not specify a version
```

---

## 📊 Impact

### Files Removed
- **23 files deleted**
- **2,996 lines removed**
- **376 lines added** (version updates, CHANGELOG, release notes)

### Clean Architecture

**Before v0.35.0:**
```
windjammer (compiler)
├── std/ui/        ❌ UI code in compiler
├── std/game/      ❌ Game code in compiler
└── runtime
    ├── ui.rs      ❌ UI runtime in compiler
    └── game.rs    ❌ Game runtime in compiler
```

**After v0.35.0:**
```
windjammer (compiler)
├── std/           ✅ Core language stdlib only
└── runtime        ✅ Core runtime only

windjammer-ui      ✅ Separate crate
└── UI framework

game-host    ✅ Separate crate
└── Game framework
```

---

## ✅ What Still Works

### Core Language Features
- ✅ Multi-target compilation (Rust, WASM, JavaScript)
- ✅ Memory safety with ownership inference
- ✅ Auto-reference insertion
- ✅ String interpolation
- ✅ Pipe operators
- ✅ LSP integration
- ✅ MCP support

### Core Stdlib Modules
- ✅ `std::fs` - File system operations
- ✅ `std::http` - HTTP client/server
- ✅ `std::json` - JSON parsing/serialization
- ✅ `std::async` - Async/await
- ✅ `std::collections` - Data structures
- ✅ `std::crypto` - Cryptography
- ✅ `std::time` - Date/time handling
- ✅ `std::process` - Process management
- ✅ `std::testing` - Test framework
- ✅ All other core modules unchanged

---

## 📦 Installation

```bash
# Via Cargo (once published)
cargo install windjammer

# Or from source
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer
cargo build --release
```

---

## 🔗 Links

- **Repository:** https://github.com/jeffreyfriedman/windjammer
- **Documentation:** https://github.com/jeffreyfriedman/windjammer/tree/main/docs
- **Related Projects:**
  - [windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui) - Cross-platform UI framework
  - [game-host](https://github.com/jeffreyfriedman/game-host) - Game development framework

---

## 📝 Migration Guide

### If You Were Using `std::ui`

**Old code (still works):**
```windjammer
use std::ui::*

fn main() {
    let button = Button::new("Click")
    button.render()
}
```

**What to change:**
```toml
# In your Cargo.toml, add:
[dependencies]
windjammer-ui = { git = "https://github.com/jeffreyfriedman/windjammer-ui" }
```

**That's it!** Your Windjammer code doesn't change.

### If You Were Using `std::game`

**Old code (still works):**
```windjammer
use std::game::*

fn main() {
    let game = Game::new()
    game.run()
}
```

**What to change:**
```toml
# In your Cargo.toml, add:
[dependencies]
game-host = { git = "https://github.com/jeffreyfriedman/game-host" }
```

**That's it!** Your Windjammer code doesn't change.

---

## 🎯 Design Philosophy

This release embodies the Windjammer philosophy:

**Before:** "Batteries included" - everything bundled together  
**After:** "Batteries available" - use what you need

- ✅ **Smaller core** - Compiler focuses on language
- ✅ **Explicit dependencies** - Clear about what you're using
- ✅ **Independent evolution** - UI and game frameworks update separately
- ✅ **No circular deps** - Clean architecture

---

## 🙏 Notes

This is a **major release with breaking changes**, but the migration is straightforward: just add the appropriate crate to your `Cargo.toml`. The Windjammer code you write stays the same.

**Why break compatibility?**
- Fixes architectural issues that would be harder to fix later
- Enables publishing to crates.io without circular dependencies
- Makes the compiler smaller and faster
- Allows UI and game frameworks to evolve independently

**What's Next:**
- 📦 Publish `windjammer` v0.35.0 to crates.io
- 📦 Publish `windjammer-ui` as a separate crate
- 📦 Publish `game-host` as a separate crate
- 📝 Update documentation with new architecture

---

**Full Changelog:** https://github.com/jeffreyfriedman/windjammer/compare/v0.34.3...v0.35.0

**Contributors:** @jeffreyfriedman

---

🎉 **Thank you for using Windjammer!**

