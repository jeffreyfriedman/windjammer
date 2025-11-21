# Game Editor Implementation Plan

## Goal
Create a fully functional game editor in pure Windjammer that allows users to:
1. Create new game projects
2. Open and edit existing games
3. Save game code
4. Run and test games
5. Configure game settings

---

## Architecture

### Web Version (WASM)
- **Location**: `examples/game_editor_functional/main.wj`
- **Compilation**: `cargo run -- run examples/game_editor_functional/main.wj --target wasm`
- **Access**: http://localhost:8080/game_editor_functional.html

### Desktop Version (Tauri)
- **Location**: `crates/game-host-editor/`
- **Backend**: Rust with Tauri commands
- **Frontend**: Pure Windjammer UI (compiled to WASM, embedded in Tauri)
- **Launch**: `cargo run -p game-host-editor`

---

## Features

### 1. Project Management
- ✅ **New Game Dialog** - Create projects with templates
  - Project name input
  - Game type selection (Platformer, Puzzle, Shooter)
  - Template selection
  
- 🔄 **Open Project** - Browse and open existing `.wj` files
  - File browser dialog
  - Recent projects list
  
- ✅ **Save Project** - Save current code to file
  - Auto-save option
  - Save as... functionality

### 2. Code Editor
- ✅ **Syntax Highlighting** - CodeEditor component
- ✅ **Line Numbers** - Built into CodeEditor
- 🔄 **Auto-completion** - Future enhancement
- 🔄 **Error Highlighting** - Show compilation errors

### 3. Game Execution
- ✅ **Run Button** - Compile and run game
  - Shows compilation progress
  - Displays game output in console
  
- ✅ **Stop Button** - Stop running game
  - Cleanup resources
  - Reset state

### 4. Settings & Configuration
- ✅ **Game Settings Panel**
  - Enable/disable sound (Checkbox)
  - Fullscreen toggle (Checkbox)
  - Volume control (Slider)
  
- ✅ **Editor Settings Dialog**
  - Dark mode toggle (Switch)
  - Auto-save toggle (Switch)
  - Theme selection (Select)

### 5. UI Layout
- ✅ **Toolbar** - Quick access to common actions
- ✅ **File Tree** - Project structure (left sidebar)
- ✅ **Code Editor** - Main editing area (center)
- ✅ **Properties Panel** - Game settings (right sidebar)
- ✅ **Console** - Output and errors (bottom)

---

## Component Usage

### Form Controls
- **Button** - Toolbar actions (New, Open, Save, Run, Stop)
- **Checkbox** - Boolean settings (Sound, Fullscreen)
- **Input** - Text input (Project name)
- **RadioGroup** - Game type selection
- **Select** - Theme selection
- **Slider** - Volume control
- **Switch** - Toggle settings (Dark mode, Auto-save)

### Layout
- **Container** - Root container
- **Flex** - Flexible layouts (toolbar, sidebars)
- **Panel** - Sectioned areas (File tree, Editor, Properties, Console)
- **Toolbar** - Button groups

### Display
- **Text** - Labels and content
- **Badge** - Status indicators (Running, Stopped)
- **Progress** - Compilation progress
- **Spinner** - Loading states

### Advanced
- **CodeEditor** - Syntax-highlighted code editing
- **Dialog** - Modal dialogs (New Game, Settings)
- **FileTree** - Project file navigation
- **Tabs** - Multiple open files

---

## Implementation Steps

### Phase 1: Basic Structure ✅
1. ✅ Create main layout with panels
2. ✅ Add toolbar with buttons
3. ✅ Integrate CodeEditor
4. ✅ Add console output

### Phase 2: Dialogs & Forms ✅
1. ✅ Implement "New Game" dialog
2. ✅ Add form controls (Input, RadioGroup)
3. ✅ Implement "Settings" dialog
4. ✅ Add settings controls (Switch, Select)

### Phase 3: Game Execution 🔄
1. ✅ Wire up Run/Stop buttons
2. 🔄 Implement compilation (call Windjammer compiler)
3. 🔄 Execute compiled game
4. 🔄 Capture and display output

### Phase 4: File Operations 🔄
1. 🔄 Implement Open dialog
2. 🔄 File browser integration
3. 🔄 Save functionality
4. 🔄 Auto-save feature

### Phase 5: Advanced Features 🔄
1. 🔄 Multiple file tabs
2. 🔄 Error highlighting
3. 🔄 Auto-completion
4. 🔄 Game preview window

---

## Desktop vs Web Differences

### Web Version (WASM)
- **Limitations**:
  - No direct file system access
  - Can't execute external processes
  - Limited to browser APIs
  
- **Solutions**:
  - Use browser File API for open/save
  - Compile Windjammer to WASM in-browser
  - Run games in iframe or separate window

### Desktop Version (Tauri)
- **Advantages**:
  - Full file system access
  - Can execute Windjammer compiler
  - Can run games as separate processes
  - Native window management
  
- **Implementation**:
  - Tauri commands for file operations
  - Process spawning for compilation/execution
  - IPC for communication

---

## Game Templates

### Platformer Template
```windjammer
use std::game::*;

fn main() {
    let player = Player::new(100, 100);
    let game = Game::new()
        .title("My Platformer")
        .size(800, 600)
        .add_entity(player);
    
    game.run();
}
```

### Puzzle Template
```windjammer
use std::game::*;

fn main() {
    let grid = Grid::new(10, 10);
    let game = Game::new()
        .title("My Puzzle")
        .size(600, 600)
        .add_grid(grid);
    
    game.run();
}
```

### Shooter Template
```windjammer
use std::game::*;

fn main() {
    let player = Ship::new(400, 500);
    let game = Game::new()
        .title("My Shooter")
        .size(800, 600)
        .add_entity(player);
    
    game.run();
}
```

---

## Testing Strategy

### Unit Tests
- Test individual components
- Test state management (Signals)
- Test event handlers

### Integration Tests
- Test dialog workflows
- Test file operations
- Test compilation pipeline

### End-to-End Tests
1. Create new game project
2. Edit code in editor
3. Save project
4. Run game
5. View output in console
6. Stop game
7. Modify settings
8. Re-run game

---

## Success Criteria

- ✅ Editor loads and displays correctly
- ✅ All buttons are clickable and responsive
- ✅ Dialogs open and close properly
- ✅ Form controls work (input, checkbox, radio, etc.)
- 🔄 Can create new game project
- 🔄 Can edit code in CodeEditor
- 🔄 Can save code to file
- 🔄 Can compile and run game
- 🔄 Console shows output
- ✅ Settings can be changed

---

## Next Steps

1. **Compile the functional editor**
   ```bash
   cd /home/user/src/windjammer
   cargo run -- run examples/game_editor_functional/main.wj --target wasm
   ```

2. **Create HTML wrapper**
   - Similar to reactive_counter.html
   - Load WASM and mount editor

3. **Test in browser**
   - Open http://localhost:8080/game_editor_functional.html
   - Verify all components render
   - Test all interactions

4. **Integrate with Tauri**
   - Update `crates/game-host-editor/ui/` with new editor
   - Wire up Tauri commands for file operations
   - Test desktop version

5. **Add real functionality**
   - Implement actual file save/load
   - Integrate Windjammer compiler
   - Execute compiled games
   - Capture and display output

---

## Documentation

- **User Guide**: How to use the editor
- **Developer Guide**: How to extend the editor
- **API Reference**: Available components and their props
- **Examples**: Sample games and tutorials

---

**Status**: Phase 1-2 complete, moving to Phase 3-4 (Game execution and file operations)

