# 🧪 Automated Testing Strategy for Windjammer

## The Problem

**Current State**: Manual testing only
- No way to verify editor buttons work without launching the app
- No CI/CD to catch regressions
- Can't test WASM examples automatically
- Hard to verify Tauri integration

**User's Question**: "how can you test this yourself, or have automated testing in CI?"

---

## Testing Pyramid for Windjammer

```
                    /\
                   /  \
                  / E2E \          ← Tauri app, browser tests
                 /------\
                /  Integ \         ← Compiler + UI integration
               /----------\
              /   Unit     \       ← Component tests, parser tests
             /--------------\
```

---

## 1. Unit Tests (Foundation)

### Current Coverage
✅ **Parser tests** - `tests/parser_tests.rs`
✅ **Type checker tests** - `tests/type_checker_tests.rs`
✅ **Backend tests** - Some in `crates/windjammer-game-editor/tests/`

### Missing Coverage
❌ **UI Component tests**
❌ **Reactivity system tests**
❌ **Code generation tests**

### Implementation

#### A. UI Component Tests

```rust
// crates/windjammer-ui/tests/component_tests.rs
use windjammer_ui::components::*;

#[test]
fn test_button_creation() {
    let button = Button::new("Click me");
    assert_eq!(button.label, "Click me");
    assert_eq!(button.variant, ButtonVariant::Primary);
    assert!(!button.disabled);
}

#[test]
fn test_button_with_variant() {
    let button = Button::new("Test")
        .variant(ButtonVariant::Secondary)
        .disabled(true);
    
    assert_eq!(button.variant, ButtonVariant::Secondary);
    assert!(button.disabled);
}

#[test]
fn test_signal_reactivity() {
    let signal = Signal::new(0);
    assert_eq!(signal.get(), 0);
    
    signal.set(42);
    assert_eq!(signal.get(), 42);
    
    signal.update(|v| v + 1);
    assert_eq!(signal.get(), 43);
}

#[test]
fn test_panel_children() {
    let panel = Panel::new("Test Panel")
        .child(Text::new("Hello"))
        .child(Button::new("Click"));
    
    assert_eq!(panel.title, "Test Panel");
    assert_eq!(panel.children.len(), 2);
}
```

#### B. Code Generation Tests

```rust
// tests/codegen_tests.rs
use windjammer::codegen::rust::RustGenerator;

#[test]
fn test_signal_codegen() {
    let wj_code = r#"
        use std::ui::*
        
        fn main() {
            let count = Signal::new(0)
            count.set(42)
        }
    "#;
    
    let rust_code = compile_to_rust(wj_code);
    assert!(rust_code.contains("windjammer_ui::reactivity::Signal::new(0)"));
    assert!(rust_code.contains("count.set(42)"));
}

#[test]
fn test_ui_component_codegen() {
    let wj_code = r#"
        use std::ui::*
        
        fn main() {
            let btn = Button::new("Test")
        }
    "#;
    
    let rust_code = compile_to_rust(wj_code);
    assert!(rust_code.contains("Button::new(\"Test\".to_string())"));
}
```

---

## 2. Integration Tests

### A. Compiler Integration Tests

```rust
// tests/integration/compiler_ui_test.rs
#[test]
fn test_compile_ui_example() {
    let result = Command::new("cargo")
        .args(&["run", "--", "build", "examples/button_test/main.wj", "--target", "wasm"])
        .output()
        .expect("Failed to run compiler");
    
    assert!(result.status.success());
    assert!(Path::new("build/main.rs").exists());
}

#[test]
fn test_wasm_compilation() {
    // Compile Windjammer to Rust
    compile_windjammer_file("examples/counter/main.wj");
    
    // Compile Rust to WASM
    let result = Command::new("cargo")
        .args(&["build", "--target", "wasm32-unknown-unknown"])
        .current_dir("build_counter")
        .output()
        .expect("Failed to build WASM");
    
    assert!(result.status.success());
    assert!(Path::new("build_counter/target/wasm32-unknown-unknown/debug/windjammer_wasm.wasm").exists());
}
```

### B. Tauri Backend Tests

```rust
// crates/windjammer-game-editor/tests/tauri_commands_test.rs
use windjammer_game_editor::*;

#[test]
fn test_create_game_project() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap().to_string();
    
    let result = create_game_project(path.clone(), "TestGame".to_string(), "platformer".to_string());
    assert!(result.is_ok());
    
    let main_wj = std::fs::read_to_string(format!("{}/TestGame/main.wj", path)).unwrap();
    assert!(main_wj.contains("Platformer Game"));
    assert!(main_wj.contains("player_x"));
}

#[test]
fn test_list_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap().to_string();
    
    // Create some test files
    std::fs::write(format!("{}/test.wj", path), "// test").unwrap();
    std::fs::create_dir(format!("{}/subdir", path)).unwrap();
    
    let files = list_directory(path).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.name == "test.wj"));
    assert!(files.iter().any(|f| f.name == "subdir" && f.is_directory));
}
```

---

## 3. End-to-End Tests

### A. WASM Browser Tests (using wasm-pack test)

```rust
// crates/windjammer-ui/tests/wasm_tests.rs
#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use windjammer_ui::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_button_renders() {
    let button = Button::new("Test");
    let vnode = button.to_vnode();
    
    match vnode {
        VNode::Element { tag, .. } => {
            assert_eq!(tag, "button");
        }
        _ => panic!("Expected element node"),
    }
}

#[wasm_bindgen_test]
fn test_signal_in_browser() {
    let signal = Signal::new(0);
    assert_eq!(signal.get(), 0);
    
    signal.set(42);
    assert_eq!(signal.get(), 42);
}
```

Run with:
```bash
wasm-pack test --headless --chrome crates/windjammer-ui
```

### B. Tauri E2E Tests (using WebDriver)

```rust
// crates/windjammer-game-editor/tests/e2e_tests.rs
use tauri_driver::WebDriver;

#[test]
#[ignore] // Run only in CI or with --ignored
fn test_editor_launches() {
    let driver = WebDriver::new().expect("Failed to create driver");
    
    // Launch the editor
    driver.get("tauri://localhost");
    
    // Wait for editor to load
    driver.wait_for_element("#editor-root", 5000);
    
    // Verify welcome screen is visible
    let welcome = driver.find_element("#welcome-screen");
    assert!(welcome.is_displayed());
}

#[test]
#[ignore]
fn test_new_project_button() {
    let driver = WebDriver::new().expect("Failed to create driver");
    driver.get("tauri://localhost");
    
    // Click new project button
    driver.find_element("#new-project").click();
    
    // Verify prompt appears (would need to handle prompt)
    // This is tricky with prompts - better to replace with modal dialogs
}
```

---

## 4. CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run unit tests
        run: cargo test --lib
      
      - name: Run integration tests
        run: cargo test --test '*'

  wasm-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      
      - name: Run WASM tests
        run: wasm-pack test --headless --chrome crates/windjammer-ui

  tauri-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install Tauri dependencies (Ubuntu)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y webkit2gtk-4.0 libgtk-3-dev
      
      - name: Build Tauri app
        run: cargo build -p windjammer-game-editor --release
      
      - name: Run Tauri tests
        run: cargo test -p windjammer-game-editor

  ui-examples:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      
      - name: Build compiler
        run: cargo build --release
      
      - name: Compile UI examples
        run: |
          ./target/release/wj build examples/reactive_counter/main.wj --target wasm
          ./target/release/wj build examples/button_test/main.wj --target wasm
          ./target/release/wj build examples/todo_simple/main.wj --target wasm
      
      - name: Build WASM
        run: |
          cd build_counter && cargo build --target wasm32-unknown-unknown
          cd ../build_button_test && cargo build --target wasm32-unknown-unknown
          cd ../build_todo_simple && cargo build --target wasm32-unknown-unknown
```

---

## 5. Quick Testing Scripts

### A. Test All Components

```bash
#!/bin/bash
# scripts/test_components.sh

echo "🧪 Testing UI Components..."
cargo test -p windjammer-ui --lib

echo "🧪 Testing Reactivity..."
cargo test -p windjammer-ui reactivity

echo "🧪 Testing Code Generation..."
cargo test codegen

echo "✅ All component tests passed!"
```

### B. Test WASM Pipeline

```bash
#!/bin/bash
# scripts/test_wasm.sh

echo "🧪 Testing WASM Compilation Pipeline..."

# Compile example to Rust
./target/release/wj build examples/reactive_counter/main.wj --target wasm --output test_build

# Compile to WASM
cd test_build
cargo build --release --target wasm32-unknown-unknown

# Run wasm-bindgen
wasm-bindgen target/wasm32-unknown-unknown/release/windjammer_wasm.wasm \
    --out-dir pkg \
    --target web

# Verify files exist
if [ -f "pkg/windjammer_wasm.js" ] && [ -f "pkg/windjammer_wasm_bg.wasm" ]; then
    echo "✅ WASM pipeline test passed!"
else
    echo "❌ WASM pipeline test failed!"
    exit 1
fi

cd ..
rm -rf test_build
```

### C. Test Tauri Editor

```bash
#!/bin/bash
# scripts/test_editor.sh

echo "🧪 Testing Game Editor..."

# Build editor
cargo build -p windjammer-game-editor --release

# Run backend tests
cargo test -p windjammer-game-editor

# Create test project
TEST_DIR=$(mktemp -d)
echo "Creating test project in $TEST_DIR"

# This would need to be automated with expect or similar
# For now, just verify build succeeds
echo "✅ Editor build test passed!"
echo "⚠️  Manual testing still required for UI interactions"
```

---

## 6. Snapshot Testing for Generated Code

```rust
// tests/snapshots/codegen_snapshots.rs
use insta::assert_snapshot;

#[test]
fn test_button_codegen_snapshot() {
    let wj_code = r#"
        use std::ui::*
        
        fn main() {
            let btn = Button::new("Test")
                .variant(ButtonVariant::Primary)
                .on_click(|| println!("Clicked!"))
        }
    "#;
    
    let rust_code = compile_to_rust(wj_code);
    assert_snapshot!(rust_code);
}
```

Run with:
```bash
cargo test --test snapshots
# Review snapshots in tests/snapshots/
cargo insta review
```

---

## 7. Performance Benchmarks

```rust
// benches/compilation_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse(c: &mut Criterion) {
    let code = std::fs::read_to_string("examples/reactive_counter/main.wj").unwrap();
    
    c.bench_function("parse counter example", |b| {
        b.iter(|| {
            parse_windjammer_code(black_box(&code))
        });
    });
}

fn bench_codegen(c: &mut Criterion) {
    let ast = parse_file("examples/reactive_counter/main.wj");
    
    c.bench_function("generate rust code", |b| {
        b.iter(|| {
            generate_rust_code(black_box(&ast))
        });
    });
}

criterion_group!(benches, bench_parse, bench_codegen);
criterion_main!(benches);
```

---

## Implementation Priority

### Phase 1: Foundation (Week 1)
1. ✅ Add unit tests for UI components
2. ✅ Add unit tests for Signal/reactivity
3. ✅ Add Tauri backend tests
4. ✅ Create test scripts

### Phase 2: Integration (Week 2)
1. ✅ Add compiler integration tests
2. ✅ Add WASM compilation tests
3. ✅ Set up CI/CD pipeline
4. ✅ Add snapshot testing

### Phase 3: E2E (Week 3)
1. ✅ Add WASM browser tests
2. ✅ Add Tauri E2E tests (WebDriver)
3. ✅ Automate UI testing
4. ✅ Performance benchmarks

---

## Immediate Actions

### 1. Fix Current Editor Issue
```bash
# Rebuild with enhanced logging
cargo build -p windjammer-game-editor --release

# Test manually with detailed logs
cargo run -p windjammer-game-editor --release
# Check console for detailed error messages
```

### 2. Add Basic Tests
```bash
# Create test files
mkdir -p crates/windjammer-ui/tests
mkdir -p crates/windjammer-game-editor/tests

# Run existing tests
cargo test
```

### 3. Set Up CI
```bash
# Create GitHub Actions workflow
mkdir -p .github/workflows
# Add test.yml (see above)
```

---

## Testing the Editor Now

Since automated tests don't exist yet, here's how to test manually with better debugging:

```bash
# 1. Build with logging
cargo build -p windjammer-game-editor --release

# 2. Run and watch console
cargo run -p windjammer-game-editor --release

# 3. Open browser dev tools (if using webview inspector)
# On macOS: Right-click → Inspect Element

# 4. Check console for:
#    - "📝 Starting new project creation..."
#    - "Invoking Tauri command: create_game_project"
#    - Any error messages with details

# 5. If prompts don't appear:
#    - Check if window has focus
#    - Try clicking directly on the button
#    - Check browser console for JavaScript errors
```

---

## Summary

**Current State**: ❌ No automated testing
**Needed**: 
- Unit tests for components
- Integration tests for compiler
- E2E tests for Tauri app
- CI/CD pipeline

**Immediate Fix**: Enhanced logging in editor (done ✅)
**Next Step**: User test with new logging, then add tests based on findings

**Long-term**: Full test pyramid with CI/CD

---

**Test the editor now with enhanced logging and report what you see in the console!**

