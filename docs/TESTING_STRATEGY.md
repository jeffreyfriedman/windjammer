# Comprehensive Testing Strategy for Windjammer

**Related:** [`USER_LIBRARY_TEST_GAPS.md`](USER_LIBRARY_TEST_GAPS.md) — why `--module-file` / outbound-`build/` dogfood libraries still use Rust `cargo test` instead of `wj test`.

## 🎯 Overview

**Goal:** Ensure all Windjammer components work reliably across platforms

**Scope:**
- Compiler (Windjammer → Rust)
- Game Framework (2D/3D rendering, physics, audio)
- Web Editor (WASM-based)
- Desktop Editor (Tauri-based)
- Web Export (games → WASM)

---

## 🧪 Testing Pyramid

```
         ┌─────────────┐
         │   Manual    │  5%
         │   Testing   │
         ├─────────────┤
         │     E2E     │  15%
         │   Testing   │
         ├─────────────┤
         │ Integration │  30%
         │   Testing   │
         ├─────────────┤
         │    Unit     │  50%
         │   Testing   │
         └─────────────┘
```

---

## 1️⃣ Unit Testing

### **Compiler Tests**

**Location:** `src/*/tests/`

**What to Test:**
- Lexer (tokenization)
- Parser (AST generation)
- Analyzer (type checking, ownership inference)
- Codegen (Rust code generation)

**Example:**
```rust
#[test]
fn test_lexer_tokenizes_function() {
    let source = "fn main() {}";
    let tokens = Lexer::new(source).tokenize();
    assert_eq!(tokens.len(), 6);
    assert_eq!(tokens[0].kind, TokenKind::Fn);
}

#[test]
fn test_parser_parses_function() {
    let source = "fn main() {}";
    let ast = Parser::new(source).parse().unwrap();
    assert_eq!(ast.functions.len(), 1);
}

#[test]
fn test_analyzer_infers_ownership() {
    let source = "fn foo(x: int) { x += 1 }";
    let analyzed = Analyzer::new().analyze(source).unwrap();
    assert_eq!(analyzed.ownership["x"], OwnershipMode::MutBorrowed);
}
```

**Current Status:**
- ✅ Lexer: ~50 tests
- ✅ Parser: ~100 tests
- ⚠️ Analyzer: ~30 tests (need more)
- ⚠️ Codegen: ~20 tests (need more)

---

### **Game Framework Tests**

**Location:** `crates/windjammer-game-framework/src/*/tests/`

**What to Test:**
- Math (Vec2, Vec3, Mat4)
- Renderer (2D/3D)
- Input (keyboard, mouse)
- Physics (collisions, raycasts)
- Audio (playback, spatial audio)

**Example:**
```rust
#[test]
fn test_vec2_add() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let c = a + b;
    assert_eq!(c, Vec2::new(4.0, 6.0));
}

#[test]
fn test_input_key_pressed() {
    let mut input = Input::new();
    input.simulate_key_press(Key::W);
    assert!(input.held(Key::W));
    assert!(input.pressed(Key::W));
}

#[test]
fn test_renderer_clear() {
    // Headless rendering test
    let renderer = Renderer::new_headless(800, 600);
    renderer.clear(Color::black());
    // Verify framebuffer is black
}
```

**Current Status:**
- ✅ Math: ~20 tests
- ⚠️ Renderer: ~5 tests (need more)
- ✅ Input: ~10 tests
- ⚠️ Physics: ~5 tests (need more)
- ⚠️ Audio: ~3 tests (need more)

---

### **Editor Tests**

**Location:** `crates/windjammer-editor-*/src/tests/`

**What to Test:**
- File operations (read, write, list)
- Project management (create, open, save)
- Compiler integration (compile, check)

**Example:**
```rust
#[test]
fn test_read_file() {
    let content = read_file("test.wj").unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_compile_windjammer() {
    let source = "fn main() {}";
    let result = compile_windjammer(source).unwrap();
    assert!(result.contains("fn main"));
}
```

**Current Status:**
- ⚠️ Web Editor: ~5 tests (need more)
- ⚠️ Desktop Editor: ~5 tests (need more)

---

## 2️⃣ Integration Testing

### **Compiler Integration Tests**

**What to Test:**
- Full compilation pipeline (Windjammer → Rust → Binary)
- Generated code compiles
- Generated code runs correctly

**Example:**
```rust
#[test]
fn test_compile_and_run_hello_world() {
    let source = r#"
        fn main() {
            println("Hello, World!")
        }
    "#;
    
    // Compile to Rust
    let rust_code = compile_to_rust(source).unwrap();
    
    // Write to temp file
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), rust_code).unwrap();
    
    // Compile Rust code
    let output = Command::new("rustc")
        .arg(temp_dir.path().join("main.rs"))
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Run binary
    let output = Command::new(temp_dir.path().join("main"))
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, World!\n");
}
```

**Current Status:**
- ⚠️ Need ~20 integration tests

---

### **Game Framework Integration Tests**

**What to Test:**
- Full game loop (init → update → render)
- Decorator-based games work
- Input handling works
- Rendering produces correct output

**Example:**
```rust
#[test]
fn test_game_loop_runs() {
    let config = GameLoopConfig::default()
        .headless()
        .with_max_frames(60);
    
    let game = TestGame::new();
    run_game_loop(game, config).unwrap();
    
    // Verify game ran for 60 frames
    assert_eq!(game.frame_count, 60);
}
```

**Current Status:**
- ⚠️ Need ~10 integration tests

---

## 3️⃣ End-to-End Testing

### **Editor E2E Tests**

**What to Test:**
- User workflows (create project, write code, compile, run)
- UI interactions (buttons, menus, dialogs)
- Cross-platform compatibility

**Tools:**
- **Web Editor**: Playwright, Puppeteer
- **Desktop Editor**: Tauri's testing framework

**Example (Web Editor with Playwright):**
```javascript
test('can create and compile project', async ({ page }) => {
    await page.goto('http://localhost:8080');
    
    // Click "New Project"
    await page.click('#new-project');
    
    // Type code
    await page.fill('#code-editor', 'fn main() { println("Hello!") }');
    
    // Click "Run"
    await page.click('#run-project');
    
    // Verify compilation success
    await expect(page.locator('#status-text')).toContainText('Compilation successful');
});
```

**Example (Desktop Editor with Tauri):**
```rust
#[tauri::test]
async fn test_compile_project() {
    let app = tauri::test::mock_app();
    
    // Invoke compile command
    let result: Result<String, String> = app
        .invoke("compile_windjammer", to_value(&CompileArgs {
            source: "fn main() {}".to_string()
        }).unwrap())
        .await;
    
    assert!(result.is_ok());
}
```

**Current Status:**
- ⚠️ Web Editor: 0 E2E tests (need to set up)
- ⚠️ Desktop Editor: 0 E2E tests (need to set up)

---

### **Game Export E2E Tests**

**What to Test:**
- Games compile to WASM
- WASM runs in browser
- Performance is acceptable

**Example:**
```rust
#[test]
fn test_export_pong_to_web() {
    // Compile PONG to web
    let output = Command::new("wj")
        .args(&["build", "examples/games/pong/main.wj", "--target=web"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Verify WASM file exists
    assert!(Path::new("build/pkg/pong_bg.wasm").exists());
    
    // Verify bundle size is reasonable
    let size = fs::metadata("build/pkg/pong_bg.wasm").unwrap().len();
    assert!(size < 5_000_000); // < 5MB
}
```

**Current Status:**
- ⚠️ Need ~5 E2E tests

---

## 4️⃣ Manual Testing

### **Cross-Platform Testing**

**Platforms to Test:**
- **Desktop**: Windows 10/11, macOS 12+, Ubuntu 22.04+
- **Web**: Chrome, Firefox, Safari, Edge
- **Mobile**: iOS 15+, Android 10+ (future)

**Test Matrix:**
```
┌─────────────┬─────────┬─────────┬─────────┬─────────┐
│             │ Windows │  macOS  │  Linux  │   Web   │
├─────────────┼─────────┼─────────┼─────────┼─────────┤
│ Compiler    │    ✅   │    ✅   │    ✅   │   N/A   │
│ Game Frmwrk │    ✅   │    ✅   │    ✅   │    ✅   │
│ Web Editor  │   N/A   │   N/A   │   N/A   │    ✅   │
│ Desktop Ed  │    ✅   │    ✅   │    ✅   │   N/A   │
│ Web Export  │    ✅   │    ✅   │    ✅   │    ✅   │
└─────────────┴─────────┴─────────┴─────────┴─────────┘
```

---

### **Performance Testing**

**Metrics to Track:**
- Compilation time (< 1s for small projects)
- Game FPS (60 FPS target)
- Memory usage (< 100MB for simple games)
- Bundle size (< 5MB for WASM)
- Load time (< 5s for web games)

**Tools:**
- `cargo bench` - Rust benchmarks
- Chrome DevTools - Web performance
- Tauri DevTools - Desktop performance

**Example:**
```rust
#[bench]
fn bench_compile_hello_world(b: &mut Bencher) {
    let source = "fn main() { println(\"Hello!\") }";
    b.iter(|| {
        compile_to_rust(source).unwrap();
    });
}
```

---

### **User Acceptance Testing (UAT)**

**Test Scenarios:**
1. **New User** - Can they create their first game?
2. **Game Jam** - Can they build and deploy quickly?
3. **Educator** - Can they teach with it?
4. **Indie Dev** - Can they build a real game?

**Feedback Channels:**
- Discord community
- GitHub issues
- User surveys
- Beta testing program

---

## 🔧 How Tauri Tests Things

### **Tauri Testing Framework**

Tauri provides several testing approaches:

#### **1. Command Testing**
```rust
#[tauri::test]
async fn test_read_file_command() {
    let app = tauri::test::mock_app();
    
    let result: Result<String, String> = app
        .invoke("read_file", to_value(&ReadFileArgs {
            path: "test.txt".to_string()
        }).unwrap())
        .await;
    
    assert!(result.is_ok());
}
```

#### **2. WebDriver Testing**
```javascript
// Using WebdriverIO
describe('Tauri App', () => {
    it('should open window', async () => {
        const app = await remote({
            capabilities: {
                'tauri:options': {
                    application: './target/release/my-app'
                }
            }
        });
        
        const title = await app.getTitle();
        expect(title).toBe('Windjammer Editor');
    });
});
```

#### **3. Integration Testing**
```rust
#[test]
fn test_app_builds() {
    tauri::test::assert_build_success();
}

#[test]
fn test_app_runs() {
    let app = tauri::test::spawn_app();
    assert!(app.is_running());
    app.kill();
}
```

---

## 📊 Testing Tools & Infrastructure

### **CI/CD Pipeline**

**GitHub Actions Workflow:**
```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run tests
        run: cargo test --workspace
      
      - name: Run integration tests
        run: cargo test --test '*' --release
      
      - name: Run benchmarks
        run: cargo bench --no-run
```

---

### **Testing Tools**

| Tool | Purpose | Status |
|------|---------|--------|
| **cargo test** | Unit tests | ✅ Active |
| **cargo bench** | Benchmarks | ⏳ Setup needed |
| **Playwright** | Web E2E | ⏳ Setup needed |
| **Tauri test** | Desktop E2E | ⏳ Setup needed |
| **wasm-pack test** | WASM tests | ⏳ Setup needed |
| **cargo-tarpaulin** | Code coverage | ⏳ Setup needed |

---

## 🎯 Testing Roadmap

### **Phase 1: Foundation** (Current)
- [x] Basic unit tests (compiler, game framework)
- [ ] Set up CI/CD (GitHub Actions)
- [ ] Code coverage tracking

### **Phase 2: Integration** (Next Month)
- [ ] Compiler integration tests
- [ ] Game framework integration tests
- [ ] Automated build testing

### **Phase 3: E2E** (Q2 2025)
- [ ] Web editor E2E tests (Playwright)
- [ ] Desktop editor E2E tests (Tauri)
- [ ] Cross-platform testing

### **Phase 4: Performance** (Q2 2025)
- [ ] Performance benchmarks
- [ ] Load testing
- [ ] Stress testing

---

## 📈 Success Metrics

### **Coverage Targets**
- Unit tests: > 80% coverage
- Integration tests: > 60% coverage
- E2E tests: Critical paths covered

### **Performance Targets**
- Compilation: < 1s for small projects
- Game FPS: 60 FPS
- Bundle size: < 5MB WASM
- Load time: < 5s web

### **Reliability Targets**
- CI/CD: All tests pass
- Flaky tests: < 1%
- Bug escape rate: < 5%

---

## 🚀 Quick Start: Running Tests

### **Run All Tests**
```bash
cargo test --workspace
```

### **Run Specific Tests**
```bash
# Compiler tests
cargo test -p windjammer

# Game framework tests
cargo test -p windjammer-game-framework

# Editor tests
cargo test -p windjammer-editor-desktop
```

### **Run Integration Tests**
```bash
cargo test --test '*' --release
```

### **Run Benchmarks**
```bash
cargo bench
```

### **Generate Coverage Report**
```bash
cargo tarpaulin --out Html
```

---

## 🎉 Summary

**Testing Strategy:**
1. ✅ **Unit Tests** - 50% (foundation in place)
2. ⏳ **Integration Tests** - 30% (need to build)
3. ⏳ **E2E Tests** - 15% (need to set up)
4. ⏳ **Manual Tests** - 5% (ongoing)

**Tauri Testing:**
- ✅ Command testing (Rust)
- ⏳ WebDriver testing (JS)
- ⏳ Integration testing

**Next Steps:**
1. Set up CI/CD (GitHub Actions)
2. Add more unit tests (80% coverage)
3. Build integration test suite
4. Set up E2E testing (Playwright + Tauri)

---

**"Test early, test often, ship confidently!"** ✅

