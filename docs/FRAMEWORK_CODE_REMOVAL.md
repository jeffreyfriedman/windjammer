# Framework Code Removal from Core Compiler

**Date:** December 14, 2025  
**Status:** ✅ Complete  
**Impact:** **-524 lines** (6381 → 5857 lines in `generator.rs`)

---

## Summary

Removed all game framework, UI framework, and platform API detection/generation logic from the core Windjammer compiler. This code was application-level functionality that didn't belong in the language compiler itself.

## What Was Removed

### 1. Framework Struct Definitions (32 lines)
- `GameFrameworkInfo` - Tracked `@game`, `@init`, `@render`, etc. decorators
- `UIFrameworkInfo` - Tracked `std::ui` usage
- `PlatformApis` - Tracked `std::fs`, `std::process`, etc. usage

### 2. Framework Detection Functions (319 lines)
- `detect_game_framework()` - Scanned for game decorators
- `detect_ui_framework()` - Scanned for UI imports
- `detect_platform_apis()` - Scanned for platform API imports
- `detect_game_import()` - Scanned for `std::game` imports
- `generate_game_main()` - Generated custom game loop main function

### 3. Framework Usage in `generate_program()` (173 lines)
- Framework detection calls
- `game_functions` HashSet creation and population
- Early generation of game-decorated functions
- Skipping `main()` for games
- Generating custom game main function
- Conditional import injection for:
  - Game framework imports (`windjammer_game_framework`)
  - UI framework imports (`windjammer_ui`)
  - Platform API imports (`windjammer_runtime::platform`)

## Why This Was Removed

### Violation of Compiler Philosophy

The core Windjammer compiler should **only** translate Windjammer syntax to Rust. It should **not**:

- ❌ Detect application-level patterns (game loops, UI components)
- ❌ Generate application-level boilerplate (main functions, ECS setup)
- ❌ Inject implicit imports based on usage patterns
- ❌ Make assumptions about application architecture

### Where This Logic Should Live

**Application-level frameworks should be separate crates:**

- `windjammer-game` - Game engine with explicit APIs
- `windjammer-ui` - UI framework with explicit setup
- `windjammer-runtime` - Platform abstraction layer

**Users should explicitly:**
- Import what they need (`use windjammer_game::prelude::*;`)
- Call setup functions (`Game::new().run()`)
- Write their own `main()` function

### Benefits of Removal

✅ **Cleaner separation of concerns** - Compiler does language, frameworks do apps  
✅ **More explicit** - No magic behavior, clear imports  
✅ **More flexible** - Users can use any architecture (OOP, ECS, hybrid)  
✅ **Easier to maintain** - Framework changes don't affect compiler  
✅ **Follows Rust conventions** - Explicit over implicit  
✅ **Follows Windjammer philosophy** - Compiler handles language complexity, not application complexity

## Impact

### Test Results
- **231/231 tests passing** ✅
- **No regressions** ✅
- **Compilation time unchanged** ✅

### Migration Path

Existing Windjammer code using decorators will need to:

1. **Remove decorators** (`@game`, `@init`, `@render`, etc.)
2. **Add explicit imports** (`use windjammer_game::prelude::*;`)
3. **Write explicit main** (call `Game::new().run()`)
4. **Pass functions explicitly** (to `Game::with_init(my_init)`)

This makes the code **more explicit** and **easier to understand**.

## Example Transformation

### Before (Magic Decorators)
```windjammer
@game
struct MyGame {
    score: i64,
}

@init
fn setup(game: MyGame) {
    game.score = 0;
}

@update
fn update(game: MyGame, dt: f32) {
    // Update logic
}

@render
fn render(game: MyGame, renderer: Renderer) {
    // Render logic
}
```

### After (Explicit Setup)
```windjammer
use windjammer_game::prelude::*;

struct MyGame {
    score: i64,
}

impl Game for MyGame {
    fn init(self) {
        self.score = 0;
    }
    
    fn update(self, dt: f32) {
        // Update logic
    }
    
    fn render(self, renderer: Renderer) {
        // Render logic
    }
}

fn main() {
    MyGame { score: 0 }.run();
}
```

**Result:** More explicit, more flexible, follows standard Rust patterns.

## Files Changed

- `windjammer/src/codegen/rust/generator.rs` - **-524 lines**
  - Removed struct definitions (lines 7-37)
  - Removed detection functions (lines 379-696)
  - Removed framework usage in `generate_program()` (scattered throughout)

## Next Steps

1. **Update `windjammer-game`** to provide explicit APIs (no decorator magic)
2. **Update `windjammer-ui`** to require explicit imports (no auto-detection)
3. **Document migration path** for existing decorator-based code
4. **Continue refactoring** `generator.rs` into smaller modules

---

## Lesson Learned

**"A compiler should compile, not make architectural decisions."**

Framework detection and code generation are **application concerns**, not **language concerns**. The core compiler should remain **lean, focused, and unopinionated** about how users structure their applications.

This removal makes Windjammer more **Rust-like** (explicit over implicit) and more **maintainable** (clear boundaries between compiler and frameworks).

---

**Commit:** `refactor: Remove framework detection from core compiler`  
**Files:** `windjammer/src/codegen/rust/generator.rs`  
**Lines:** -524  
**Tests:** 231/231 passing ✅








