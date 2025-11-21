# 🧪 Windjammer UI Framework - Testing Guide

## 🌐 Web Examples (WASM)

### Server Status
The UI server should be running on **http://localhost:8080**

If it's not running, start it with:
```bash
cd /home/user/src/windjammer/crates/windjammer-ui
cargo run --bin serve_wasm
```

### Available Examples

#### 1. Examples Index Page
**URL**: http://localhost:8080/examples/index.html

**What it is**: A beautiful landing page listing all available examples with descriptions and status.

**Features**:
- Overview of all examples
- Links to working demos
- Status of in-progress features
- Framework capabilities summary

---

#### 2. Interactive Counter ✅ WORKING
**URL**: http://localhost:8080/examples/reactive_counter.html

**What to test**:
1. Open the page
2. Click "+ Increment" button → Count should increase immediately
3. Click "- Decrement" button → Count should decrease immediately
4. Click "Reset" button → Count should go to 0
5. Watch status text change based on count value

**Expected behavior**:
- ✅ UI updates immediately on button click
- ✅ Count changes in real-time
- ✅ Status text updates automatically
- ✅ No page reload needed
- ✅ No console errors

**What this proves**:
- Reactive state management works
- Automatic UI re-rendering works
- Event handlers work
- Signal updates propagate correctly

---

#### 3. Button Test ✅ WORKING
**URL**: http://localhost:8080/examples/button_test.html

**What to test**:
1. Open the page
2. Open DevTools console (F12 or Cmd+Option+I)
3. Click the "Click Me!" button multiple times
4. Watch console output

**Expected behavior**:
- ✅ Button is clickable
- ✅ Console shows: "🎉 Button clicked! Count: 1", "Count: 2", etc.
- ✅ Signal value increments (visible in console)
- ✅ Event handler executes correctly

**What this proves**:
- Event handlers work
- Signals update correctly
- Console logging from WASM works
- Button component works

---

#### 4. Game Editor (Static) ✅ WORKING
**URL**: http://localhost:8080/examples/wasm_editor.html

**What to test**:
1. Open the page
2. Observe the UI layout
3. See all panels and components

**Expected behavior**:
- ✅ Full editor UI renders
- ✅ Toolbar with buttons appears
- ✅ File panel, editor panel, console panel visible
- ✅ VS Code-inspired dark theme
- ✅ Clean, professional layout

**What this proves**:
- Complex layouts work
- Panel system works
- Component composition works
- Styling system works

**Note**: This is the static version. Buttons aren't interactive yet - that's what the reactive version will add!

---

## 🖥️ Desktop Examples (Tauri)

### Desktop Game Editor
**Status**: 🔄 In Development (75% complete)

**Location**: `crates/game-host-editor`

**Current state**:
- ✅ Tauri backend ready with file operations
- ✅ HTML/JS frontend works
- 🔄 Pure Windjammer UI being integrated

**To test the current HTML/JS version**:
```bash
cd /home/user/src/windjammer/crates/game-host-editor
cargo tauri dev
```

**Expected behavior**:
- Desktop window opens
- Editor UI appears
- Can create/open projects
- File operations work
- Console output visible

**Next steps**:
- Replace HTML/JS with pure Windjammer UI
- Use ReactiveApp for interactivity
- Full dogfooding achieved!

---

## ⚠️ Known Limitations

### 1. Todo App
**Status**: Code written but needs feature adjustments

**Issue**: Uses closure parameters `|value|` which aren't fully supported yet

**Workaround**: Created simplified version, needs compilation fixes

### 2. Form Inputs
**Status**: Input component exists but needs event parameter support

**Issue**: `on_change(|value| ...)` requires closure parameter support

**Planned**: Add closure parameter support to compiler

### 3. Desktop Editor Integration
**Status**: UI code complete, needs compilation and integration

**Remaining work**:
- Compile reactive editor to WASM
- Integrate with Tauri webview
- Connect Tauri commands

---

## 🎯 What Works Right Now

### ✅ Fully Functional
1. **Reactive Counter** - Complete, production-ready example
2. **Button Test** - Event handling verification
3. **Static UI** - Complex layouts and components
4. **Signal System** - Automatic UI updates
5. **Event Handling** - Click handlers, state updates
6. **WASM Compilation** - Windjammer → Rust → WASM

### 🔄 Partially Functional
1. **Todo App** - Code written, needs adjustments
2. **Desktop Editor** - Backend ready, UI integration pending
3. **Forms** - Components exist, need closure support

### 📋 Planned
1. **Form Validation** - Real-time error messages
2. **Data Fetching** - API calls, loading states
3. **Routing** - Multi-page applications
4. **Mobile** - Tauri Mobile support

---

## 🚀 Quick Start Testing

### 1. Start the Server
```bash
cd /home/user/src/windjammer/crates/windjammer-ui
cargo run --bin serve_wasm
```

### 2. Open Your Browser
Visit: http://localhost:8080/examples/index.html

### 3. Try the Examples
- Click "Open Interactive Counter"
- Play with the buttons
- See real-time updates!

### 4. Check the Console
- Open DevTools (F12)
- See WASM loading messages
- Watch for any errors

---

## 🐛 Troubleshooting

### Server Won't Start
```bash
# Kill any process on port 8080
lsof -ti:8080 | xargs kill -9

# Start fresh
cd /home/user/src/windjammer/crates/windjammer-ui
cargo run --bin serve_wasm
```

### Page Shows "Loading..." Forever
1. Check browser console for errors
2. Verify WASM files exist in `pkg/` directory
3. Check server is running on port 8080
4. Try refreshing the page (Cmd+Shift+R or Ctrl+Shift+R)

### UI Doesn't Update on Button Click
1. This should work for Interactive Counter
2. If not, check console for JavaScript errors
3. Verify `ReactiveApp` is being used (not `App`)

### Desktop Editor Won't Launch
```bash
# Rebuild Tauri app
cd /home/user/src/windjammer/crates/game-host-editor
cargo tauri build

# Or run in dev mode
cargo tauri dev
```

---

## 📊 Testing Checklist

### Web Examples
- [ ] Server starts successfully
- [ ] Index page loads and looks good
- [ ] Interactive Counter: All buttons work
- [ ] Interactive Counter: UI updates in real-time
- [ ] Button Test: Console shows click events
- [ ] Static Editor: UI renders correctly
- [ ] No console errors
- [ ] Performance is smooth

### Desktop (When Ready)
- [ ] Desktop app launches
- [ ] File operations work
- [ ] Project creation works
- [ ] Code editor works
- [ ] Run/stop game works
- [ ] Console output appears

---

## 🎉 Success Criteria

### For Web Examples
✅ Interactive Counter works perfectly
✅ UI updates automatically
✅ No console errors
✅ Performance is smooth
✅ Professional appearance

### For Desktop
🔄 App launches successfully
🔄 Pure Windjammer UI renders
🔄 File operations work
🔄 Full dogfooding achieved

---

## 📝 Feedback

When testing, note:
1. What works well
2. What doesn't work
3. Any errors in console
4. Performance issues
5. UI/UX improvements

---

**Ready to test?** Start here: http://localhost:8080/examples/index.html

**Questions?** Check the docs in `/home/user/src/windjammer/docs/`

