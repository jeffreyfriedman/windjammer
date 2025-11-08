# 🧪 Windjammer Manual Testing Guide

**Complete guide for testing all Windjammer features**

This guide provides step-by-step instructions for manually testing all Windjammer components to ensure production readiness.

---

## 📋 **Pre-Testing Checklist**

Before starting manual tests, ensure:

- [ ] Windjammer compiler is built: `cargo build --release`
- [ ] `wj` binary is in PATH: `which wj`
- [ ] LSP server is available: `which windjammer-lsp`
- [ ] All crates compile: `cargo build --workspace`
- [ ] All tests pass: `cargo test --workspace`

---

## 1️⃣ **Core Compiler Testing**

### Test 1.1: Basic Compilation

```bash
# Create test file
cat > test_basic.wj << 'EOF'
fn main() {
    let x = 42
    println!("Hello, Windjammer! x = {}", x)
}
EOF

# Compile
wj build test_basic.wj --output test_output

# Run
cd test_output && cargo run
```

**Expected Output**: `Hello, Windjammer! x = 42`

**✅ Pass Criteria**: Compiles and runs successfully

---

### Test 1.2: Auto-Clone System

```bash
cat > test_auto_clone.wj << 'EOF'
fn process(data: Vec<int>) -> int {
    data.len()
}

fn main() {
    let data = vec![1, 2, 3]
    let len = process(data)
    println!("Length: {}", len)
    println!("Data still works: {:?}", data)  // Should work!
}
EOF

wj build test_auto_clone.wj --output test_auto_clone_output
cd test_auto_clone_output && cargo run
```

**Expected Output**:
```
Length: 3
Data still works: [1, 2, 3]
```

**✅ Pass Criteria**: No manual `.clone()` needed, data still usable

---

## 2️⃣ **Error System Testing**

### Test 2.1: Error Messages

```bash
cat > test_error.wj << 'EOF'
fn main() {
    let x = 42
    println!("{}", missing_variable)
}
EOF

wj build test_error.wj --check
```

**Expected Output**:
```
error[WJ0002]: Variable not found: missing_variable
  --> test_error.wj:3:20
   |
 3 |     println!("{}", missing_variable)
   |                    ^^^^^^^^^^^^^^^^ not found in this scope
   |
   = help: Did you mean `x`?
   = note: Variables must be declared before use
   💡 wj explain WJ0002
```

**✅ Pass Criteria**: 
- Error code `WJ0002` displayed
- Contextual help provided
- Fuzzy matching suggests `x`

---

### Test 2.2: Auto-Fix

```bash
cat > test_autofix.wj << 'EOF'
fn main() {
    let count = 0
    count = count + 1
    println!("{}", count)
}
EOF

wj build test_autofix.wj --check --fix
```

**Expected Output**:
```
✓ Fixed: Made variable 'count' mutable
✓ Compilation successful!
```

**✅ Pass Criteria**: Auto-fixes mutability error

---

### Test 2.3: Interactive TUI

```bash
wj errors test_error.wj
```

**Expected**: Interactive TUI opens with:
- Error list on left
- Error details on right
- Keyboard navigation (↑/↓)
- Help screen (?)

**✅ Pass Criteria**: TUI displays and navigates correctly

---

### Test 2.4: Error Statistics

```bash
# Generate some errors first
wj build test_error.wj --check
wj build test_autofix.wj --check

# View stats
wj stats
```

**Expected Output**:
```
╭────────────────────────────────────────────────╮
│  Windjammer Error Statistics                  │
╰────────────────────────────────────────────────╯

Total Compilations: 2
Total Errors: 2
Error Rate: 1.00 errors/compilation

Most Common Errors:
  1. WJ0002 (Variable not found): 1 occurrences
  2. WJ0004 (Mutability error): 1 occurrences
```

**✅ Pass Criteria**: Statistics displayed correctly

---

### Test 2.5: Error Catalog

```bash
wj docs --format markdown
cat docs/errors/errors.md | head -50
```

**Expected**: Markdown file with error documentation

**✅ Pass Criteria**: Catalog generated successfully

---

### Test 2.6: Explain Command

```bash
wj explain WJ0002
```

**Expected Output**:
```
╭────────────────────────────────────────────────╮
│  Error Code: WJ0002                           │
│  Variable not found                           │
╰────────────────────────────────────────────────╯

What This Means:
  The compiler cannot find a variable with this name...
```

**✅ Pass Criteria**: Detailed explanation displayed

---

## 3️⃣ **LSP Server Testing**

### Test 3.1: Start LSP Server

```bash
windjammer-lsp
```

**Expected**: Server starts and waits for connections

**✅ Pass Criteria**: No errors, server runs

---

### Test 3.2: VS Code Extension (Manual)

1. **Install Extension**:
   ```bash
   cd vscode-extension
   npm install
   npm run compile
   code --install-extension .
   ```

2. **Test Features**:
   - Open a `.wj` file
   - Check syntax highlighting
   - Hover over a function (should show signature)
   - Press F12 on a symbol (go to definition)
   - Type and check autocomplete (Ctrl+Space)
   - Check inlay hints (ownership modes)

**✅ Pass Criteria**: All LSP features work

---

## 4️⃣ **Windjammer-UI Testing**

### Test 4.1: Build UI Crate

```bash
cd crates/windjammer-ui
cargo build --release
```

**Expected**: Builds successfully

**✅ Pass Criteria**: No compilation errors

---

### Test 4.2: Run UI Tests

```bash
cargo test
```

**Expected Output**:
```
running 8 tests
iii.....
test result: ok. 5 passed; 0 failed; 3 ignored
```

**✅ Pass Criteria**: All non-ignored tests pass

---

### Test 4.3: WASM Counter Example

```bash
cd crates/windjammer-ui
wasm-pack build --target web

# Serve the example
python3 -m http.server 8080
```

Then open `http://localhost:8080/examples/counter_wasm.html`

**Expected**: Counter app loads and works:
- Click "+" button → count increases
- Click "-" button → count decreases
- Reactive updates work

**✅ Pass Criteria**: Counter app works in browser

---

## 5️⃣ **Windjammer-Game-Framework Testing**

### Test 5.1: Build Game Framework

```bash
cd crates/windjammer-game-framework
cargo build --release
```

**Expected**: Builds successfully

**✅ Pass Criteria**: No compilation errors

---

### Test 5.2: Run Game Tests

```bash
cargo test
```

**Expected Output**:
```
running 25 tests
.........................
test result: ok. 25 passed; 0 failed; 0 ignored
```

**✅ Pass Criteria**: All tests pass

---

### Test 5.3: Window Test Example

```bash
cargo run --example window_test
```

**Expected**: 
- Window opens (800x600)
- White background
- Window closes cleanly

**✅ Pass Criteria**: Window displays correctly

---

### Test 5.4: Sprite Test Example

```bash
cargo run --example sprite_test
```

**Expected**:
- Window opens
- Blue square sprite visible
- Sprite moves smoothly
- No flickering

**✅ Pass Criteria**: Sprite renders and moves

---

### Test 5.5: Physics Test Example

```bash
cargo run --example physics_test
```

**Expected**:
- Window opens
- Falling squares visible
- Physics simulation runs (gravity, bouncing)
- Smooth 60 FPS

**✅ Pass Criteria**: Physics works correctly

---

### Test 5.6: Game Loop Test Example

```bash
cargo run --example game_loop_test
```

**Expected**:
- Console output shows FPS
- Consistent 60 UPS (updates per second)
- Variable FPS (rendering)

**✅ Pass Criteria**: Game loop maintains 60 UPS

---

## 6️⃣ **Multi-Target Compilation Testing**

### Test 6.1: JavaScript Target

```bash
cat > test_js.wj << 'EOF'
fn greet(name: string) {
    println!("Hello, {}!", name)
}

fn main() {
    greet("JavaScript")
}
EOF

wj build --target=javascript test_js.wj --output test_js_output
node test_js_output/output.js
```

**Expected Output**: `Hello, JavaScript!`

**✅ Pass Criteria**: Compiles to JS and runs

---

### Test 6.2: WASM Target

```bash
wj build --target=wasm test_js.wj --output test_wasm_output
cd test_wasm_output
wasm-pack build --target web
```

**Expected**: WASM module builds successfully

**✅ Pass Criteria**: No errors, `.wasm` file generated

---

## 7️⃣ **Performance Testing**

### Test 7.1: Large File Compilation

```bash
# Generate large file
python3 << 'EOF'
with open('test_large.wj', 'w') as f:
    f.write('fn main() {\n')
    for i in range(1000):
        f.write(f'    let x{i} = {i}\n')
    f.write('    println!("Done")\n')
    f.write('}\n')
EOF

time wj build test_large.wj --output test_large_output
```

**Expected**: Compiles in < 5 seconds

**✅ Pass Criteria**: Reasonable compilation time

---

### Test 7.2: LSP Performance

1. Open large `.wj` file in VS Code
2. Type and check autocomplete latency
3. Hover over symbols
4. Go to definition

**Expected**: All operations < 100ms

**✅ Pass Criteria**: No noticeable lag

---

## 8️⃣ **Integration Testing**

### Test 8.1: Full Project Build

```bash
# Create multi-file project
mkdir test_project
cd test_project

cat > main.wj << 'EOF'
use lib

fn main() {
    let result = lib::add(2, 3)
    println!("Result: {}", result)
}
EOF

cat > lib.wj << 'EOF'
fn add(a: int, b: int) -> int {
    a + b
}
EOF

wj build main.wj --output output
cd output && cargo run
```

**Expected Output**: `Result: 5`

**✅ Pass Criteria**: Multi-file project works

---

### Test 8.2: Stdlib Usage

```bash
cat > test_stdlib.wj << 'EOF'
use std::fs
use std::path

fn main() {
    let path = path::join("test", "file.txt")
    println!("Path: {}", path)
}
EOF

wj build test_stdlib.wj --output test_stdlib_output
cd test_stdlib_output && cargo run
```

**Expected Output**: `Path: test/file.txt`

**✅ Pass Criteria**: Stdlib modules work

---

## 9️⃣ **Edge Cases Testing**

### Test 9.1: Empty File

```bash
touch test_empty.wj
wj build test_empty.wj --check
```

**Expected**: Error about missing `main` function

**✅ Pass Criteria**: Handles gracefully

---

### Test 9.2: Unicode Support

```bash
cat > test_unicode.wj << 'EOF'
fn main() {
    let emoji = "🚀"
    let chinese = "你好"
    println!("{} {}", emoji, chinese)
}
EOF

wj build test_unicode.wj --output test_unicode_output
cd test_unicode_output && cargo run
```

**Expected Output**: `🚀 你好`

**✅ Pass Criteria**: Unicode works correctly

---

## 🎯 **Final Verification Checklist**

After completing all tests, verify:

- [ ] Core compiler works (Tests 1.1-1.2)
- [ ] Error system works (Tests 2.1-2.6)
- [ ] LSP server works (Tests 3.1-3.2)
- [ ] UI crate works (Tests 4.1-4.3)
- [ ] Game framework works (Tests 5.1-5.6)
- [ ] Multi-target works (Tests 6.1-6.2)
- [ ] Performance acceptable (Tests 7.1-7.2)
- [ ] Integration works (Tests 8.1-8.2)
- [ ] Edge cases handled (Tests 9.1-9.2)

---

## 📊 **Test Results Template**

Use this template to record your test results:

```
Date: ___________
Tester: ___________
Version: 0.35.0

Core Compiler:
- Test 1.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 1.2: [ ] Pass [ ] Fail - Notes: ___________

Error System:
- Test 2.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 2.2: [ ] Pass [ ] Fail - Notes: ___________
- Test 2.3: [ ] Pass [ ] Fail - Notes: ___________
- Test 2.4: [ ] Pass [ ] Fail - Notes: ___________
- Test 2.5: [ ] Pass [ ] Fail - Notes: ___________
- Test 2.6: [ ] Pass [ ] Fail - Notes: ___________

LSP Server:
- Test 3.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 3.2: [ ] Pass [ ] Fail - Notes: ___________

Windjammer-UI:
- Test 4.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 4.2: [ ] Pass [ ] Fail - Notes: ___________
- Test 4.3: [ ] Pass [ ] Fail - Notes: ___________

Windjammer-Game-Framework:
- Test 5.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 5.2: [ ] Pass [ ] Fail - Notes: ___________
- Test 5.3: [ ] Pass [ ] Fail - Notes: ___________
- Test 5.4: [ ] Pass [ ] Fail - Notes: ___________
- Test 5.5: [ ] Pass [ ] Fail - Notes: ___________
- Test 5.6: [ ] Pass [ ] Fail - Notes: ___________

Multi-Target:
- Test 6.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 6.2: [ ] Pass [ ] Fail - Notes: ___________

Performance:
- Test 7.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 7.2: [ ] Pass [ ] Fail - Notes: ___________

Integration:
- Test 8.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 8.2: [ ] Pass [ ] Fail - Notes: ___________

Edge Cases:
- Test 9.1: [ ] Pass [ ] Fail - Notes: ___________
- Test 9.2: [ ] Pass [ ] Fail - Notes: ___________

Overall Status: [ ] Production Ready [ ] Needs Work

Issues Found:
1. ___________
2. ___________
3. ___________
```

---

## 🚀 **Production Readiness Criteria**

Windjammer is **production ready** when:

✅ **All core tests pass** (Tests 1.x)  
✅ **Error system fully functional** (Tests 2.x)  
✅ **LSP provides good DX** (Tests 3.x)  
✅ **UI crate works** (Tests 4.x)  
✅ **Game framework works** (Tests 5.x)  
✅ **Multi-target compiles** (Tests 6.x)  
✅ **Performance acceptable** (Tests 7.x)  
✅ **Integration solid** (Tests 8.x)  
✅ **Edge cases handled** (Tests 9.x)

---

## 📞 **Reporting Issues**

If you find issues during testing:

1. Note the test number
2. Record exact steps to reproduce
3. Include error messages
4. Note your environment (OS, Rust version)
5. File an issue on GitHub

---

**Happy Testing! 🧪**

**Windjammer is ready to change the world! 🚀**

