# Windjammer Game Editor Testing Strategy

## Overview

This document outlines the testing strategy for the Windjammer Game Editor, which is built using `windjammer-ui` for dogfooding purposes.

## Architecture

The Game Editor consists of:
1. **Tauri Backend** (`crates/windjammer-game-editor/src/main.rs`) - Rust backend providing file system access and compiler integration
2. **Windjammer UI Frontend** (`crates/windjammer-game-editor/ui/editor.wj`) - UI built with windjammer-ui, compiled to WASM
3. **Windjammer Compiler Integration** - Ability to compile and run games from within the editor

## Testing Phases

### Phase 1: Component Testing ✅
**Status**: COMPLETE

**Goal**: Verify that individual windjammer-ui components work correctly

**Tests**:
- ✅ Button component creation and variants
- ✅ Text component with different sizes
- ✅ Container layout
- ✅ Panel component
- ✅ Flex layout with direction and gap
- ⚠️ CodeEditor (needs Signal support - see Phase 4)
- ⚠️ FileTree (needs implementation)

**Test File**: `examples/ui_test_simple/main.wj`

**Results**:
```
Testing UI components...
✓ Button created
✓ Text created
✓ Container created
✓ Panel created
✓ CodeEditor (skipped - needs Signal)
✓ Flex created
✅ All UI components work!
```

### Phase 2: Backend Testing
**Status**: PENDING

**Goal**: Verify Tauri backend commands work correctly

**Tests**:
1. **File Operations**
   - `read_file()` - Read Windjammer source files
   - `write_file()` - Save edited files
   - `list_directory()` - Browse project files
   
2. **Project Management**
   - `create_game_project()` - Create new game from template
   - Verify template structure is correct
   - Verify main.wj has valid game code

3. **Compiler Integration**
   - `run_game()` - Compile and run a game
   - `stop_game()` - Stop running game
   - Verify compiler output is captured
   - Verify error messages are displayed

**Test Approach**:
```bash
# Build the editor backend
cd crates/windjammer-game-editor
cargo build

# Run manual tests
cargo test
```

### Phase 3: UI Integration Testing
**Status**: PENDING

**Goal**: Verify the complete UI renders and interacts correctly

**Tests**:
1. **Layout Rendering**
   - Toolbar renders with all buttons
   - Main area shows file tree, editor, and preview
   - Console panel displays output
   
2. **User Interactions**
   - Click "New Project" button
   - Select file from file tree
   - Edit code in editor
   - Click "Save" button
   - Click "Run" button
   - View console output

**Test Approach**:
```bash
# Compile UI to WASM
cd crates/windjammer-game-editor/ui
windjammer build editor.wj --target wasm

# Run the editor
cd ..
cargo run
```

### Phase 4: Signal and Reactivity Testing
**Status**: PENDING

**Goal**: Implement and test reactive state management

**Issues to Resolve**:
- CodeEditor expects `Signal<String>` but stdlib defines `string`
- Need to expose Signal type in Windjammer stdlib
- Need codegen support for Signal creation

**Required Changes**:
1. Add `Signal<T>` type to `std/ui/mod.wj`
2. Update codegen to handle Signal creation
3. Update CodeEditor stdlib definition to use Signal

**Test Code**:
```windjammer
use std::ui::*

fn main() {
    let content = Signal::new("fn main() {}".to_string())
    let editor = CodeEditor::new(content)
        .language("windjammer")
        .on_change(|new_content| {
            println!("Code changed: {}", new_content)
        })
}
```

### Phase 5: End-to-End Testing
**Status**: PENDING

**Goal**: Test complete workflow from project creation to running game

**Workflow**:
1. Launch Game Editor
2. Click "New Project"
3. Enter project name "TestGame"
4. Select project location
5. Editor creates project with template
6. File tree shows main.wj
7. Click main.wj to open in editor
8. Edit game code (change player color)
9. Click "Save"
10. Click "Run"
11. Game compiles successfully
12. Game window opens and runs
13. Verify player moves with arrow keys
14. Click "Stop"
15. Game window closes
16. Edit code again (add obstacle)
17. Click "Run"
18. Verify changes are reflected

**Success Criteria**:
- ✅ All steps complete without errors
- ✅ File operations work correctly
- ✅ Compiler integration works
- ✅ Game runs and responds to input
- ✅ Console shows appropriate messages
- ✅ UI remains responsive throughout

### Phase 6: Error Handling Testing
**Status**: PENDING

**Goal**: Verify error cases are handled gracefully

**Test Cases**:
1. **File Errors**
   - Try to open non-existent file
   - Try to save to read-only location
   - Try to create project in invalid location

2. **Compilation Errors**
   - Introduce syntax error in code
   - Verify error message is displayed in console
   - Verify line number is highlighted (if supported)

3. **Runtime Errors**
   - Game crashes during execution
   - Verify error is caught and displayed
   - Verify editor remains functional

## Test Automation

### Unit Tests
```bash
# Test individual components
cd crates/windjammer-game-editor
cargo test
```

### Integration Tests
```bash
# Test full editor workflow
cd crates/windjammer-game-editor
cargo test --test integration
```

### Manual Testing Checklist

- [ ] Editor launches successfully
- [ ] All UI components render correctly
- [ ] File tree shows project structure
- [ ] Code editor displays syntax highlighting
- [ ] Can create new project
- [ ] Can open existing project
- [ ] Can edit and save files
- [ ] Can compile game
- [ ] Can run game
- [ ] Can stop game
- [ ] Console shows output
- [ ] Errors are displayed clearly
- [ ] UI is responsive and doesn't freeze
- [ ] Can switch between multiple files
- [ ] Changes are saved correctly

## Known Issues

1. **CodeEditor Signal Support**
   - CodeEditor requires Signal<String> but stdlib uses string
   - Need to add Signal support to stdlib and codegen

2. **FileTree Implementation**
   - FileTree component needs full implementation
   - Need to integrate with Tauri backend

3. **Syntax Highlighting**
   - CodeEditor needs Windjammer syntax highlighting
   - May need to integrate with Monaco or CodeMirror

4. **Game Preview**
   - Preview panel currently just shows text
   - Need to embed game window or show screenshot

## Next Steps

1. ✅ Complete Phase 1 (Component Testing)
2. ⏭️ Implement Phase 2 (Backend Testing)
3. ⏭️ Implement Phase 3 (UI Integration)
4. ⏭️ Resolve Signal support (Phase 4)
5. ⏭️ Complete E2E testing (Phase 5)
6. ⏭️ Error handling (Phase 6)

## Success Metrics

- **Functional**: All core features work (create, edit, save, run)
- **Reliable**: No crashes or data loss
- **Performant**: UI responds within 100ms
- **Usable**: Clear error messages and intuitive workflow
- **Dogfooding**: Validates windjammer-ui design and API

