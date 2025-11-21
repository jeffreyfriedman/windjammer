# Windjammer Editor Status

## Overview

Windjammer has **two editors** in development:
1. **Desktop Editor** (egui-based, native)
2. **Browser Editor** (HTML/WASM-based, web)

Both editors are **partially complete** and need significant work to become fully functional.

---

## Desktop Editor (`game-host-editor`)

**Location**: `crates/game-host-editor/`  
**Technology**: egui + egui_dock + wgpu  
**Status**: 🚧 **PARTIAL** - Core works, many panels incomplete

### ✅ What Works:
- **Core Editor Framework**: Docking system, menu bar, panels
- **File Management**: Project browser, file tree navigation
- **Code Editor**: Basic syntax highlighting
- **Scene Editor**: 3D scene visualization with wgpu
- **Properties Panel**: Basic property editing
- **Console**: Build output and logs
- **PBR Material Editor**: ✅ Complete
- **Post-Processing Editor**: ✅ Complete
- **Performance Profiler**: ✅ Complete
- **Particle System Editor**: ✅ Complete

### 🚧 What's Incomplete:
- **Animation Editor**: ✅ UI complete, needs runtime integration
- **Terrain Editor**: ✅ UI complete, needs runtime integration
- **AI Behavior Tree Editor**: ✅ UI complete, needs runtime integration
- **Audio Mixer**: ✅ UI complete, needs runtime integration
- **Gamepad Config**: ✅ UI complete, needs runtime integration
- **Weapon Editor**: ✅ UI complete, needs runtime integration
- **NavMesh Editor**: ✅ UI complete, needs runtime integration
- **Asset Browser**: ❌ Not implemented (file browser only)
- **Transform Gizmos**: ❌ Not implemented in 3D scene view
- **Play Mode**: ❌ Not implemented

**Note**: All 11 panels have complete UI implementations! They just need integration with the actual game framework runtime.

### Running the Desktop Editor:

```bash
cd /path/to/windjammer
cargo run --package game-host-editor --bin editor --features desktop --release
```

---

## Browser Editor (`windjammer-editor-web`)

**Location**: `crates/windjammer-editor-web/`  
**Technology**: HTML/CSS/JavaScript + WASM + WebGL  
**Status**: 🔴 **NON-FUNCTIONAL** - UI prototype only

### ✅ What's Complete (Infrastructure):
- **WASM Build System**: Engine compiles to WASM ✅
- **Storage System**: IndexedDB/localStorage infrastructure ✅
- **WebGL Renderer**: 3D PBR rendering works (standalone) ✅
- **UI Layout**: HTML/CSS panels designed ✅

### 🚧 What's Partial (UI Only):
- **Scene Editor**: Has visual layout but NO functionality
- **Hierarchy Panel**: Shows UI but can't manage entities
- **Inspector Panel**: Shows UI but can't edit components
- **Viewport**: WebGL renderer exists but NOT integrated with editor
- **Console**: Shows UI but not connected to anything

### 🔴 What's Missing (Critical):
1. **WASM Integration**: WASM engine not connected to HTML UI
2. **Entity Management**: Can't create/delete/select entities
3. **Component Editing**: Can't add/edit/remove components
4. **Scene Serialization**: Can't actually save/load scenes
5. **Viewport Integration**: WebGL renderer not in editor viewport
6. **Event Handling**: No mouse/keyboard interaction
7. **State Management**: No editor state synchronization

### Current State:

The browser editor is currently **just a static HTML page** with panels. It looks like an editor but doesn't function as one. All the pieces exist (WASM, storage, WebGL) but they're not connected.

### Running the Browser Editor:

```bash
cd crates/windjammer-editor-web
./build.sh  # Build WASM
./serve.sh  # Start local server
# Open http://localhost:8080
```

**Note**: You'll see the UI but it won't do anything yet.

---

## Migration to windjammer-ui

### Current Situation:
- **Desktop Editor**: Uses `egui` (external library)
- **Browser Editor**: Uses raw HTML/CSS/JavaScript
- **Problem**: Two completely different codebases, no code sharing

### Goal:
Migrate both editors to use **windjammer-ui** (our own UI framework) so they can:
- Share the same component code
- Have consistent behavior
- Support both desktop and browser from one codebase
- Use Windjammer's declarative UI syntax

### Migration Plan:

#### Phase 1: Core Components
1. Create windjammer-ui components for:
   - Panel system
   - Docking layout
   - Menu bar
   - Tree view (hierarchy)
   - Property grid (inspector)
   - Console output
   - Viewport container

#### Phase 2: Desktop Migration
1. Replace egui panels with windjammer-ui components
2. Keep wgpu for 3D rendering
3. Test all existing functionality
4. Ensure no regressions

#### Phase 3: Browser Migration
1. Replace HTML/CSS with windjammer-ui components
2. Integrate WASM engine
3. Connect WebGL renderer
4. Implement editor functionality

#### Phase 4: Unification
1. Extract shared panel logic
2. Create platform-specific rendering backends
3. Single codebase, dual targets
4. Unified feature development

---

## Feature Comparison

| Feature | Desktop Editor | Browser Editor | Target |
|---------|---------------|----------------|--------|
| **Core Framework** | ✅ Working | 🔴 Non-functional | Both |
| **Scene Hierarchy** | ✅ Working | 🔴 UI only | Both |
| **Entity Inspector** | ✅ Working | 🔴 UI only | Both |
| **3D Viewport** | ✅ Working | 🔴 Not integrated | Both |
| **Asset Browser** | 🚧 Partial | 🔴 Missing | Both |
| **Transform Gizmos** | 🔴 Missing | 🔴 Missing | Both |
| **Play Mode** | 🔴 Missing | 🔴 Missing | Both |
| **PBR Material Editor** | ✅ Complete | 🔴 Missing | Both |
| **Post-Processing** | ✅ Complete | 🔴 Missing | Both |
| **Particle Editor** | ✅ Complete | 🔴 Missing | Both |
| **Animation Editor** | 🚧 Partial | 🔴 Missing | Both |
| **Terrain Editor** | 🚧 Partial | 🔴 Missing | Both |
| **AI Behavior Tree** | 🚧 Partial | 🔴 Missing | Both |
| **Audio Mixer** | 🚧 Partial | 🔴 Missing | Both |
| **Profiler** | ✅ Complete | 🔴 Missing | Both |
| **Code Editor** | ✅ Basic | 🔴 Missing | Desktop only |
| **Project Management** | ✅ Working | 🔴 Missing | Both |

---

## Priority Tasks

### Immediate (Make Browser Editor Functional):
1. 🔴 **CRITICAL**: Integrate WASM engine with HTML UI
2. 🔴 **CRITICAL**: Implement entity creation/deletion
3. 🔴 **CRITICAL**: Implement component editing
4. 🔴 **CRITICAL**: Implement scene save/load
5. 🔴 **CRITICAL**: Connect WebGL renderer to viewport

### Short-term (Complete Desktop Editor):
1. 🎨 Complete animation editor panel
2. 🎨 Complete terrain editor panel
3. 🎨 Complete AI behavior tree editor
4. 🎨 Complete audio mixer panel
5. 🎨 Add transform gizmos to 3D view
6. 🎨 Implement play mode

### Medium-term (Migration):
1. 🏗️ Design windjammer-ui editor components
2. 🏗️ Migrate desktop editor to windjammer-ui
3. 🏗️ Migrate browser editor to windjammer-ui
4. 🏗️ Unify codebases with shared components

### Long-term (Polish):
1. ✨ Visual scripting
2. ✨ Collaborative editing
3. ✨ Cloud storage integration
4. ✨ Asset marketplace integration
5. ✨ Plugin system integration

---

## Technical Debt

### Desktop Editor:
- **egui Dependency**: External library, limits customization
- **Separate Panels**: Each panel is mostly independent
- **No Shared State**: Limited state management between panels
- **Platform Specific**: Can't easily port to web

### Browser Editor:
- **Non-functional**: Needs complete implementation
- **Raw HTML/CSS**: Hard to maintain, no component reuse
- **No Integration**: WASM, storage, WebGL all separate
- **No State Management**: No editor state system

### Both Editors:
- **Code Duplication**: Will duplicate features if not unified
- **Different APIs**: Inconsistent user experience
- **Maintenance Burden**: Two codebases to maintain
- **Feature Parity**: Hard to keep features in sync

---

## Recommendations

### For Users:
- **Use Desktop Editor** for now (it actually works)
- **Browser Editor** is not ready for use yet
- Expect significant changes as we migrate to windjammer-ui

### For Contributors:
- **Desktop Editor**: Focus on completing existing panels
- **Browser Editor**: Focus on making it functional first
- **Migration**: Wait for windjammer-ui component design
- **New Features**: Consider both editors when designing

### For Project Planning:
1. **Priority 1**: Make browser editor functional (critical gap)
2. **Priority 2**: Complete desktop editor panels (user value)
3. **Priority 3**: Design windjammer-ui editor components (foundation)
4. **Priority 4**: Migrate both editors (unification)
5. **Priority 5**: Add advanced features (polish)

---

## Timeline Estimate

### Browser Editor Functionality: 2-3 weeks
- WASM integration: 3-5 days
- Entity management: 3-5 days
- Component editing: 3-5 days
- Scene serialization: 2-3 days
- Testing and polish: 2-3 days

### Desktop Editor Completion: 3-4 weeks
- Animation editor: 4-5 days
- Terrain editor: 4-5 days
- AI behavior tree: 4-5 days
- Audio mixer: 3-4 days
- Gizmos and play mode: 5-7 days
- Testing and polish: 3-4 days

### Migration to windjammer-ui: 6-8 weeks
- Component design: 1-2 weeks
- Desktop migration: 2-3 weeks
- Browser migration: 2-3 weeks
- Unification and testing: 1-2 weeks

**Total**: ~3-4 months to have two fully functional, unified editors

---

## Resources

- [Desktop Editor README](../crates/game-host-editor/README.md)
- [Browser Editor WASM Guide](../crates/windjammer-editor-web/WASM_BUILD.md)
- [windjammer-ui Documentation](../crates/windjammer-ui/README.md)
- [Project Roadmap](../ROADMAP.md)

---

*Last Updated: November 20, 2024*
