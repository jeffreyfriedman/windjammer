# Windjammer Native Testing Framework

## 🎯 Philosophy

**"Test Windjammer with Windjammer"**

Instead of relying on Rust's `#[test]` or JavaScript testing frameworks, we should have a **native Windjammer testing framework** that:
1. Tests are written in pure Windjammer
2. Tests compile and run like any other Windjammer code
3. Tests exercise the full compiler pipeline
4. Tests validate that Windjammer itself works correctly

---

## 🧪 Test Syntax Design

### **Basic Test**

```windjammer
@test
fn test_addition() {
    let result = 2 + 2
    assert_eq(result, 4)
}

@test
fn test_string_concat() {
    let greeting = "Hello, " + "World!"
    assert_eq(greeting, "Hello, World!")
}
```

### **Test with Setup/Teardown**

```windjammer
@test_setup
fn setup() {
    println("Setting up test environment...")
}

@test_teardown
fn teardown() {
    println("Cleaning up...")
}

@test
fn test_with_setup() {
    let x = 10
    assert_eq(x, 10)
}
```

### **Test Groups/Modules**

```windjammer
@test_module("Math Tests")
mod math_tests {
    @test
    fn test_add() {
        assert_eq(2 + 2, 4)
    }
    
    @test
    fn test_subtract() {
        assert_eq(5 - 3, 2)
    }
    
    @test
    fn test_multiply() {
        assert_eq(3 * 4, 12)
    }
}

@test_module("String Tests")
mod string_tests {
    @test
    fn test_length() {
        let s = "hello"
        assert_eq(s.len(), 5)
    }
    
    @test
    fn test_concat() {
        let s = "hello" + " world"
        assert_eq(s, "hello world")
    }
}
```

### **Assertions**

```windjammer
// Basic assertions
assert(condition)                    // Assert true
assert_eq(actual, expected)          // Assert equal
assert_ne(actual, expected)          // Assert not equal
assert_gt(actual, expected)          // Assert greater than
assert_lt(actual, expected)          // Assert less than
assert_gte(actual, expected)         // Assert >=
assert_lte(actual, expected)         // Assert <=

// String assertions
assert_contains(string, substring)   // Contains substring
assert_starts_with(string, prefix)   // Starts with
assert_ends_with(string, suffix)     // Ends with

// Collection assertions
assert_empty(collection)             // Is empty
assert_not_empty(collection)         // Not empty
assert_length(collection, expected)  // Has length

// Error assertions
assert_throws(fn, error_type)        // Function throws error
assert_no_throw(fn)                  // Function doesn't throw

// Custom message
assert_eq(actual, expected, "Custom error message")
```

### **Async Tests**

```windjammer
@test
@async
fn test_async_operation() {
    let result = await fetch_data()
    assert_eq(result, "data")
}
```

### **Parameterized Tests**

```windjammer
@test
@params([
    (2, 2, 4),
    (3, 3, 9),
    (4, 4, 16),
])
fn test_multiply(a: int, b: int, expected: int) {
    let result = a * b
    assert_eq(result, expected)
}
```

### **Benchmark Tests**

```windjammer
@benchmark
fn bench_fibonacci() {
    let result = fibonacci(20)
    assert_eq(result, 6765)
}

@benchmark
@iterations(1000)
fn bench_string_concat() {
    let s = "hello" + " world"
}
```

---

## 🏗️ Implementation Architecture

```
┌─────────────────────────────────────┐
│    Test File (test_math.wj)        │
│  @test fn test_add() { ... }        │
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│      Windjammer Compiler            │
│  - Detects @test decorators         │
│  - Generates test harness           │
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│         Rust Code (main.rs)         │
│  - Test runner                      │
│  - Assertion helpers                │
│  - Result reporting                 │
└──────────────┬──────────────────────┘
               │
               ↓
┌─────────────────────────────────────┐
│      Compiled Binary (test)         │
│  - Runs all tests                   │
│  - Reports results                  │
└─────────────────────────────────────┘
```

---

## 🔧 Test Runner

### **Command Line Interface**

```bash
# Run all tests
wj test

# Run specific test file
wj test tests/test_math.wj

# Run specific test
wj test tests/test_math.wj::test_add

# Run tests matching pattern
wj test --filter "math"

# Run tests in parallel
wj test --parallel

# Run benchmarks
wj test --bench

# Generate coverage report
wj test --coverage

# Watch mode (re-run on file change)
wj test --watch
```

### **Output Format**

```
Running tests...

test_math.wj:
  ✓ test_add (0.1ms)
  ✓ test_subtract (0.1ms)
  ✓ test_multiply (0.1ms)
  ✗ test_divide (0.2ms)
    Expected: 2
    Actual: 2.5
    at test_math.wj:15

test_string.wj:
  ✓ test_length (0.1ms)
  ✓ test_concat (0.1ms)

Summary:
  5 passed, 1 failed, 6 total
  Time: 0.6ms
```

---

## 📝 Standard Library Tests

### **Example: Testing Vec2**

```windjammer
// std/math/vec2_test.wj

@test_module("Vec2 Tests")
mod vec2_tests {
    @test
    fn test_vec2_new() {
        let v = Vec2::new(1.0, 2.0)
        assert_eq(v.x, 1.0)
        assert_eq(v.y, 2.0)
    }
    
    @test
    fn test_vec2_add() {
        let a = Vec2::new(1.0, 2.0)
        let b = Vec2::new(3.0, 4.0)
        let c = a + b
        assert_eq(c.x, 4.0)
        assert_eq(c.y, 6.0)
    }
    
    @test
    fn test_vec2_length() {
        let v = Vec2::new(3.0, 4.0)
        assert_eq(v.length(), 5.0)
    }
    
    @test
    fn test_vec2_normalize() {
        let v = Vec2::new(3.0, 4.0)
        let n = v.normalize()
        assert_eq(n.length(), 1.0)
    }
}
```

### **Example: Testing String Operations**

```windjammer
// std/strings/string_test.wj

@test_module("String Tests")
mod string_tests {
    @test
    fn test_string_length() {
        let s = "hello"
        assert_eq(s.len(), 5)
    }
    
    @test
    fn test_string_concat() {
        let s = "hello" + " world"
        assert_eq(s, "hello world")
    }
    
    @test
    fn test_string_slice() {
        let s = "hello world"
        let sub = s.slice(0, 5)
        assert_eq(sub, "hello")
    }
    
    @test
    fn test_string_contains() {
        let s = "hello world"
        assert(s.contains("world"))
        assert(!s.contains("foo"))
    }
    
    @test
    fn test_string_split() {
        let s = "a,b,c"
        let parts = s.split(",")
        assert_eq(parts.len(), 3)
        assert_eq(parts[0], "a")
        assert_eq(parts[1], "b")
        assert_eq(parts[2], "c")
    }
}
```

---

## 🎮 Game Framework Tests

### **Example: Testing Input System**

```windjammer
// tests/game/test_input.wj

@test_module("Input Tests")
mod input_tests {
    @test
    fn test_key_pressed() {
        let input = Input::new()
        input.simulate_key_press(Key::W)
        
        assert(input.held(Key::W))
        assert(input.pressed(Key::W))
        assert(!input.released(Key::W))
    }
    
    @test
    fn test_key_released() {
        let input = Input::new()
        input.simulate_key_press(Key::W)
        input.clear_frame_state()
        input.simulate_key_release(Key::W)
        
        assert(!input.held(Key::W))
        assert(!input.pressed(Key::W))
        assert(input.released(Key::W))
    }
    
    @test
    fn test_mouse_position() {
        let input = Input::new()
        input.simulate_mouse_move(100.0, 200.0)
        
        let pos = input.mouse_position()
        assert_eq(pos.0, 100.0)
        assert_eq(pos.1, 200.0)
    }
}
```

### **Example: Testing Renderer**

```windjammer
// tests/game/test_renderer.wj

@test_module("Renderer Tests")
mod renderer_tests {
    @test
    fn test_renderer_clear() {
        let renderer = Renderer::new_headless(800, 600)
        renderer.clear(Color::black())
        
        // Verify framebuffer is black
        let pixel = renderer.get_pixel(0, 0)
        assert_eq(pixel.r, 0.0)
        assert_eq(pixel.g, 0.0)
        assert_eq(pixel.b, 0.0)
    }
    
    @test
    fn test_renderer_draw_rect() {
        let renderer = Renderer::new_headless(800, 600)
        renderer.clear(Color::white())
        renderer.draw_rect(10.0, 10.0, 50.0, 50.0, Color::red())
        
        // Verify pixel inside rect is red
        let pixel = renderer.get_pixel(20, 20)
        assert_eq(pixel.r, 1.0)
        assert_eq(pixel.g, 0.0)
        assert_eq(pixel.b, 0.0)
    }
}
```

---

## 🔬 Compiler Tests

### **Example: Testing Lexer**

```windjammer
// tests/compiler/test_lexer.wj

@test_module("Lexer Tests")
mod lexer_tests {
    @test
    fn test_tokenize_function() {
        let source = "fn main() {}"
        let tokens = Lexer::tokenize(source)
        
        assert_eq(tokens.len(), 6)
        assert_eq(tokens[0].kind, TokenKind::Fn)
        assert_eq(tokens[1].kind, TokenKind::Identifier)
        assert_eq(tokens[2].kind, TokenKind::LParen)
        assert_eq(tokens[3].kind, TokenKind::RParen)
        assert_eq(tokens[4].kind, TokenKind::LBrace)
        assert_eq(tokens[5].kind, TokenKind::RBrace)
    }
    
    @test
    fn test_tokenize_numbers() {
        let source = "42 3.14 0xFF"
        let tokens = Lexer::tokenize(source)
        
        assert_eq(tokens.len(), 3)
        assert_eq(tokens[0].kind, TokenKind::IntLiteral)
        assert_eq(tokens[1].kind, TokenKind::FloatLiteral)
        assert_eq(tokens[2].kind, TokenKind::IntLiteral)
    }
}
```

### **Example: Testing Parser**

```windjammer
// tests/compiler/test_parser.wj

@test_module("Parser Tests")
mod parser_tests {
    @test
    fn test_parse_function() {
        let source = "fn add(a: int, b: int) -> int { a + b }"
        let ast = Parser::parse(source)
        
        assert_eq(ast.functions.len(), 1)
        
        let func = ast.functions[0]
        assert_eq(func.name, "add")
        assert_eq(func.params.len(), 2)
        assert_eq(func.return_type, Type::Int)
    }
    
    @test
    fn test_parse_struct() {
        let source = "struct Point { x: float, y: float }"
        let ast = Parser::parse(source)
        
        assert_eq(ast.structs.len(), 1)
        
        let s = ast.structs[0]
        assert_eq(s.name, "Point")
        assert_eq(s.fields.len(), 2)
    }
}
```

---

## 🚀 Integration Tests

### **Example: Full Compilation Test**

```windjammer
// tests/integration/test_compile.wj

@test_module("Compilation Tests")
mod compile_tests {
    @test
    fn test_compile_hello_world() {
        let source = "fn main() { println(\"Hello, World!\") }"
        
        // Compile to Rust
        let rust_code = Compiler::compile(source)
        assert(rust_code.contains("fn main"))
        assert(rust_code.contains("println!"))
        
        // Compile Rust to binary
        let binary = RustCompiler::compile(rust_code)
        assert(binary.is_ok())
        
        // Run binary
        let output = binary.run()
        assert_eq(output, "Hello, World!\n")
    }
    
    @test
    fn test_compile_game() {
        let source = """
        @game
        struct MyGame {
            score: int,
        }
        
        @init
        fn init(game: MyGame) {
            game.score = 0
        }
        """
        
        let rust_code = Compiler::compile(source)
        assert(rust_code.contains("struct MyGame"))
        assert(rust_code.contains("fn init"))
    }
}
```

---

## 📊 Test Coverage

### **Coverage Report**

```windjammer
// Generate coverage report
wj test --coverage

// Output:
Coverage Report:
  std/math/vec2.wj: 95% (38/40 lines)
  std/strings/string.wj: 87% (52/60 lines)
  src/lexer.wj: 92% (184/200 lines)
  src/parser.wj: 78% (312/400 lines)
  
  Total: 85% (586/700 lines)
```

---

## 🎯 Benefits of Windjammer-Native Testing

### **1. Self-Validation**
- Tests prove Windjammer works
- Tests exercise the full compiler
- Tests validate language features

### **2. Dogfooding**
- We use our own language
- We find issues early
- We improve the language

### **3. Documentation**
- Tests serve as examples
- Tests show best practices
- Tests demonstrate features

### **4. Consistency**
- Same syntax everywhere
- No context switching
- Easier to learn

### **5. Integration**
- Tests compile like code
- Tests run like code
- Tests deploy like code

---

## 🛠️ Implementation Plan

### **Phase 1: Core Framework** (Next Week)
- [ ] Add `@test` decorator support
- [ ] Implement assertion functions
- [ ] Generate test harness
- [ ] Basic test runner

### **Phase 2: Advanced Features** (Next Month)
- [ ] Test modules/groups
- [ ] Setup/teardown
- [ ] Parameterized tests
- [ ] Async tests

### **Phase 3: Tooling** (Q2 2025)
- [ ] Coverage reporting
- [ ] Benchmark support
- [ ] Watch mode
- [ ] Parallel execution

### **Phase 4: Integration** (Q2 2025)
- [ ] CI/CD integration
- [ ] IDE support
- [ ] Test generation
- [ ] Mutation testing

---

## 📝 Standard Library Test Structure

```
std/
├── math/
│   ├── vec2.wj
│   ├── vec2_test.wj      ← Tests for Vec2
│   ├── vec3.wj
│   └── vec3_test.wj      ← Tests for Vec3
├── strings/
│   ├── string.wj
│   └── string_test.wj    ← Tests for String
├── collections/
│   ├── vec.wj
│   └── vec_test.wj       ← Tests for Vec
└── game/
    ├── input.wj
    ├── input_test.wj     ← Tests for Input
    ├── renderer.wj
    └── renderer_test.wj  ← Tests for Renderer
```

---

## 🎉 Example: Complete Test File

```windjammer
// tests/test_math.wj

@test_module("Math Operations")
mod math_tests {
    @test_setup
    fn setup() {
        println("Setting up math tests...")
    }
    
    @test_teardown
    fn teardown() {
        println("Cleaning up math tests...")
    }
    
    @test
    fn test_addition() {
        assert_eq(2 + 2, 4)
        assert_eq(10 + 5, 15)
        assert_eq(-5 + 3, -2)
    }
    
    @test
    fn test_subtraction() {
        assert_eq(5 - 3, 2)
        assert_eq(10 - 15, -5)
    }
    
    @test
    fn test_multiplication() {
        assert_eq(3 * 4, 12)
        assert_eq(5 * 0, 0)
        assert_eq(-2 * 3, -6)
    }
    
    @test
    fn test_division() {
        assert_eq(10 / 2, 5)
        assert_eq(15 / 3, 5)
    }
    
    @test
    @params([
        (0, 1),
        (1, 1),
        (2, 2),
        (3, 6),
        (4, 24),
        (5, 120),
    ])
    fn test_factorial(n: int, expected: int) {
        let result = factorial(n)
        assert_eq(result, expected)
    }
    
    @benchmark
    fn bench_fibonacci() {
        let result = fibonacci(20)
        assert_eq(result, 6765)
    }
}

fn factorial(n: int) -> int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

fn fibonacci(n: int) -> int {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}
```

---

## 🚀 Running Tests

```bash
# Run all tests
wj test

# Output:
Running tests in tests/test_math.wj...

Math Operations:
  ✓ test_addition (0.1ms)
  ✓ test_subtraction (0.1ms)
  ✓ test_multiplication (0.1ms)
  ✓ test_division (0.1ms)
  ✓ test_factorial (0.5ms)
    - (0, 1) ✓
    - (1, 1) ✓
    - (2, 2) ✓
    - (3, 6) ✓
    - (4, 24) ✓
    - (5, 120) ✓

Benchmarks:
  bench_fibonacci: 1.2ms (avg over 1000 iterations)

Summary:
  5 tests passed, 0 failed
  1 benchmark completed
  Total time: 2.1ms
```

---

## 🎯 Success Metrics

### **Coverage Targets**
- Standard library: > 90%
- Compiler: > 80%
- Game framework: > 85%

### **Performance Targets**
- Test execution: < 100ms for unit tests
- Full test suite: < 5s

### **Quality Targets**
- All tests pass on all platforms
- No flaky tests
- Clear error messages

---

## 🎉 Summary

**Windjammer-Native Testing:**
- ✅ Tests written in pure Windjammer
- ✅ Self-validating (tests prove Windjammer works)
- ✅ Dogfooding (we use our own language)
- ✅ Consistent (same syntax everywhere)
- ✅ Integrated (tests compile like code)

**Implementation:**
- Phase 1: Core framework (next week)
- Phase 2: Advanced features (next month)
- Phase 3: Tooling (Q2 2025)
- Phase 4: Integration (Q2 2025)

**Status:** Design complete, ready to implement!

---

**"Test Windjammer with Windjammer!"** ✅

