# 🎯 Pure Windjammer Game Editor - Implementation Plan

## Goal

Rewrite the game editor in **100% pure Windjammer** with ZERO HTML/CSS/JavaScript!

---

## Current Status

✅ **Created**: `crates/windjammer-game-editor/ui/editor_pure.wj`
- 100% Windjammer code
- No HTML, no CSS, no JavaScript
- Uses `windjammer-ui` components
- Uses `std::tauri` API (not direct Tauri calls)

---

## What's in editor_pure.wj

### 1. State Management (Signals)
```windjammer
let project_name = Signal::new("")
let project_path = Signal::new("/tmp")
let template = Signal::new("platformer")
let code_content = Signal::new("...")
let console_output = Signal::new("...")
let show_new_project_dialog = Signal::new(false)
```

### 2. Reactive UI
```windjammer
ReactiveApp::new("Windjammer Game Editor", move || {
    Container::new()
        .child(create_toolbar(...))
        .child(create_main_area(...))
        .child(create_new_project_dialog(...))
        .child(create_open_project_dialog(...))
}).run()
```

### 3. Components Used
- `Container` - Layout
- `Panel` - Sections with titles
- `Flex` - Flexbox layout
- `Button` - Actions
- `Input` - Text input
- `CodeEditor` - Code editing
- `Text` - Display text
- `Dialog` - Modal dialogs

### 4. Tauri Integration
```windjammer
tauri::create_game_project(path, name, template)
tauri::read_file(file_path)
tauri::write_file(file_path, content)
tauri::run_game(project_path)
tauri::stop_game()
```

**NO direct Tauri calls!** All through `std::tauri` API.

---

## Implementation Steps

### Step 1: Enhance std::tauri API ✅
**File**: `std/tauri/mod.wj`

Add all needed Tauri functions:
- `create_game_project`
- `read_file`
- `write_file`
- `list_directory`
- `run_game`
- `stop_game`

### Step 2: Improve Dialog Component
**File**: `crates/windjammer-ui/src/components/dialog.rs`

Add:
- `on_confirm` callback
- `on_cancel` callback
- `visible` property
- Proper modal rendering

### Step 3: Improve Input Component
**File**: `crates/windjammer-ui/src/components/input.rs`

Add:
- `label` method
- `placeholder` method
- Two-way binding with Signal

### Step 4: Compile to WASM
```bash
cargo run --release -- build crates/windjammer-game-editor/ui/editor_pure.wj --target wasm --output build_editor_pure
cd build_editor_pure
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/windjammer_wasm.wasm --out-dir pkg --target web
```

### Step 5: Minimal HTML Wrapper
**File**: `crates/windjammer-game-editor/ui/index_minimal.html`

```html
<!DOCTYPE html>
<html>
<head>
    <title>Windjammer Game Editor</title>
    <link rel="stylesheet" href="../../../crates/windjammer-ui/examples/components.css">
</head>
<body>
    <div id="app"></div>
    <script type="module">
        import init, { start } from './pkg/windjammer_wasm.js';
        await init();
        start();
    </script>
</body>
</html>
```

**That's it!** Just loads the WASM, no other HTML/CSS/JS!

### Step 6: Update Tauri Config
**File**: `crates/windjammer-game-editor/tauri.conf.json`

```json
{
  "build": {
    "frontendDist": "./ui/pkg"
  }
}
```

### Step 7: Delete Old Files
```bash
rm crates/windjammer-game-editor/ui/index.html
rm crates/windjammer-game-editor/ui/app.js
rm crates/windjammer-game-editor/ui/styles.css
rm crates/windjammer-game-editor/ui/dialog.html
```

### Step 8: Test
```bash
cargo run -p windjammer-game-editor --release
```

---

## Challenges & Solutions

### Challenge 1: Dialog Component
**Problem**: Current `Dialog` doesn't have callbacks
**Solution**: Add `on_confirm` and `on_cancel` methods

### Challenge 2: Input Binding
**Problem**: Input needs two-way binding with Signal
**Solution**: Add `value` property that takes `Signal<String>`

### Challenge 3: Conditional Rendering
**Problem**: Need to show/hide dialogs
**Solution**: Use `if show.get() { ... }` pattern

### Challenge 4: String vs bool for Signals
**Problem**: Windjammer parser might not support `Signal<bool>`
**Solution**: Use `Signal<String>` with "true"/"false" values

---

## Benefits of Pure Windjammer

### 1. Swappability ✅
Can replace Tauri with:
- Native desktop framework
- Web framework
- Custom implementation

Just swap the platform layer in `windjammer-ui`!

### 2. Maintainability ✅
- One codebase (Windjammer)
- No context switching
- Type safety throughout

### 3. Dogfooding ✅
- Proves `windjammer-ui` works
- Exposes missing features
- Validates the design

### 4. Consistency ✅
- Same patterns everywhere
- Same reactivity model
- Same component API

---

## Next Actions

1. **Enhance `std/tauri/mod.wj`** with all needed functions
2. **Improve `Dialog` component** with callbacks
3. **Improve `Input` component** with label/placeholder
4. **Compile `editor_pure.wj`** to WASM
5. **Test** the pure Windjammer editor
6. **Delete** old HTML/CSS/JS files
7. **Celebrate** 🎉

---

## Timeline

- **Step 1-3**: 1 hour (enhance components)
- **Step 4-5**: 30 minutes (compile and wrapper)
- **Step 6-7**: 15 minutes (config and cleanup)
- **Step 8**: 15 minutes (testing)

**Total**: ~2 hours

---

## Success Criteria

✅ Editor written in 100% Windjammer
✅ No HTML (except minimal wrapper)
✅ No CSS (uses components.css from windjammer-ui)
✅ No JavaScript (except WASM loader)
✅ All Tauri calls through `std::tauri` API
✅ Can create, open, edit, save, run games
✅ Fully reactive with Signals
✅ Professional UI with components

---

**Status**: Ready to implement! 🚀

