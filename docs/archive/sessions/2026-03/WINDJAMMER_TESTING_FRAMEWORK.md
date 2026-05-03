# Windjammer Testing Framework - Design Document

**Status:** Design Phase  
**Priority:** High (Post-rendering features)  
**Goal:** Self-hosted testing in Windjammer, minimal Rust

---

## 🎯 Philosophy

**Core Principle:** Testing should feel like Windjammer, not like JavaScript or Python.

- ✅ **Windjammer-native** - Use existing syntax (functions, attributes, modules)
- ✅ **Composable** - Multiple testing styles work together
- ✅ **Self-hosting** - Tests written in Windjammer (dogfooding!)
- ✅ **Zero ceremony** - Minimal boilerplate
- ✅ **TDD-first** - Test framework itself is TDD-built

---

## 📐 Design: Hybrid Approach

### 1. **Primary: Decorator-Based Tests** (90% of use cases)

```rust
// Simple test
@test
fn sprite_creation() {
    let sprite = Sprite::new();
    assert_eq(sprite.width, 32);
}

// With decorators
@test
@ignore
fn expensive_test() {
    // Skipped by default
}

@test
@timeout(5000) // milliseconds
fn should_be_fast() {
    // Must complete in 5 seconds
}

// With setup/teardown
fn setup() -> Database {
    Database::connect_test()
}

fn teardown(db: Database) {
    db.disconnect();
}

@test(setup = setup, teardown = teardown)
fn database_query(db: Database) {
    // db is automatically passed in
    assert(db.is_connected());
    let users = db.query("SELECT * FROM users");
    assert_length(users, 10);
}
```

**Why this works:**
- Feels like Windjammer (functions are first-class)
- Like Rust (familiar to target audience)
- No weird syntax (no strings as test names)
- Clean, minimal, obvious

---

### 2. **Organization: Modules** (Natural grouping)

```rust
// tests/sprite_tests.wj
mod sprite_tests {
    @test
    fn creation() {
        let sprite = Sprite::new();
        assert_eq(sprite.width, 32);
    }
    
    @test
    fn animation() {
        let sprite = Sprite::new();
        sprite.animate(0.016);
        assert_gt(sprite.current_frame, 0);
    }
    
    @test
    fn collision() {
        let sprite1 = Sprite::new();
        let sprite2 = Sprite::new();
        assert(!sprite1.collides_with(sprite2));
    }
}

// Output:
// ✓ sprite_tests::creation (1ms)
// ✓ sprite_tests::animation (2ms)
// ✓ sprite_tests::collision (1ms)
```

**Why this works:**
- Modules are Windjammer's natural organization
- Nested modules = nested test groups
- No special "describe" blocks needed
- Scoped helpers (setup functions per module)

---

### 3. **Parameterized: Table-Driven Tests**

```rust
// Syntax 1: Decorator with data
@test_cases(
    (1, 1, 2),
    (2, 3, 5),
    (-1, 1, 0),
    (100, -50, 50),
)
fn addition(a: int, b: int, expected: int) {
    assert_eq(add(a, b), expected);
}

// Syntax 2: Inline table
@test
fn vector_operations() {
    let cases = [
        (Vec2::new(0.0, 0.0), 0.0),
        (Vec2::new(3.0, 4.0), 5.0),
        (Vec2::new(1.0, 1.0), 1.414),
    ];
    
    for (vec, expected_length) in cases {
        assert_approx(vec.length(), expected_length, 0.001);
    }
}

// Output:
// ✓ addition (case 1: 1, 1 -> 2) (0ms)
// ✓ addition (case 2: 2, 3 -> 5) (0ms)
// ✓ addition (case 3: -1, 1 -> 0) (0ms)
// ✓ addition (case 4: 100, -50 -> 50) (0ms)
```

**Why this works:**
- Great for reducing duplication
- Easy to add more test cases
- Clear which case failed
- Familiar pattern (Go, Rust table tests)

---

### 4. **Property-Based: QuickCheck Style**

```rust
// Test properties, not specific cases
@property_test
fn addition_is_commutative(a: int, b: int) {
    assert_eq(a + b, b + a);
}

@property_test
fn addition_is_associative(a: int, b: int, c: int) {
    assert_eq((a + b) + c, a + (b + c));
}

// With custom generators
@property_test(gen = valid_sprites())
fn sprite_always_positive(sprite: Sprite) {
    assert_gte(sprite.x, 0.0);
    assert_gte(sprite.y, 0.0);
}

fn valid_sprites() -> Generator<Sprite> {
    Generator::new(|| {
        Sprite {
            x: random_range(0.0, 1000.0),
            y: random_range(0.0, 1000.0),
            width: random_range(1.0, 100.0),
            height: random_range(1.0, 100.0),
        }
    })
}

// Output:
// ✓ addition_is_commutative (100 cases passed) (5ms)
// ✓ addition_is_associative (100 cases passed) (8ms)
// ✓ sprite_always_positive (100 cases passed) (12ms)
```

**Why this works:**
- Catches edge cases automatically
- More thorough than example-based
- Proves properties mathematically
- Optional (use when needed)

---

### 5. **Contracts: Design-by-Contract** (Advanced)

```rust
// Pre/post conditions
@requires(x > 0)
@requires(y > 0)
@ensures(result > x)
@ensures(result > y)
fn add(x: int, y: int) -> int {
    x + y
}

// Invariants
@invariant(self.health >= 0)
@invariant(self.health <= self.max_health)
struct Character {
    health: int,
    max_health: int,
}

impl Character {
    @requires(amount > 0)
    @ensures(self.health <= old(self.health))
    fn take_damage(&mut self, amount: int) {
        self.health -= amount;
        if self.health < 0 {
            self.health = 0;
        }
    }
}

// Tests are automatically generated from contracts!
// No need to write tests for basic invariants
```

**Why this works:**
- Tests are specifications (formal verification)
- Catches bugs at compile-time or runtime
- Self-documenting (contracts are documentation)
- Optional (use for critical code)

---

### 6. **Doc Tests: Inline Examples**

```rust
/// Calculates the length of a vector.
/// 
/// # Example
/// ```test
/// let v = Vec2::new(3.0, 4.0);
/// assert_eq(v.length(), 5.0);
/// ```
fn vector_length_example() { }

/// Character takes damage.
/// 
/// # Example
/// ```test
/// let mut char = Character::new(100);
/// char.take_damage(30);
/// assert_eq(char.health, 70);
/// ```
fn character_damage_example() { }
```

**Why this works:**
- Documentation stays up-to-date (tests fail if wrong)
- Examples are executable
- No separate test files for simple cases
- Optional (use when examples are useful)

---

## 🧰 Assertion Library

### Basic Assertions

```rust
// Boolean
assert(condition);
assert(x > 0, "x must be positive");

// Equality
assert_eq(left, right);
assert_ne(left, right);

// Comparisons
assert_gt(a, b);  // greater than
assert_lt(a, b);  // less than
assert_gte(a, b); // >=
assert_lte(a, b); // <=

// Floating point
assert_approx(actual, expected, epsilon);
assert_approx(computed, 3.14159, 0.0001);

// Collections
assert_contains(collection, item);
assert_empty(collection);
assert_length(collection, 5);

// Optionals
assert_is_some(optional);
assert_is_none(optional);

// Results
assert_is_ok(result);
assert_is_err(result);

// Panics
assert_panics {
    divide_by_zero();
}

assert_panics_with("division by zero") {
    divide_by_zero();
}
```

### Advanced Assertions

```rust
// Custom predicates
assert_matches(value, pattern);
assert_matches(result, Ok(_));

// Deep equality
assert_deep_eq(obj1, obj2); // Compares all fields recursively

// Range checks
assert_in_range(value, min, max);
assert_in_range(temperature, 0.0, 100.0);

// Type checks
assert_type<T>(value);
assert_type<Sprite>(entity.get_component());
```

---

## 🔧 Mocking System

### Interface Mocking

```rust
// Define interface
trait Database {
    fn query(sql: string) -> Vec<Row>;
    fn execute(sql: string) -> Result<(), Error>;
}

// In tests
#[test]
fn user_service_test() {
    // Create mock
    let mut mock_db = mock<Database>();
    
    // Set expectations
    mock_db.expect_query("SELECT * FROM users")
           .returns(vec![Row::new("alice"), Row::new("bob")]);
    
    mock_db.expect_execute("INSERT INTO users")
           .returns(Ok(()));
    
    // Use mock
    let service = UserService::new(mock_db);
    let users = service.get_all_users();
    
    // Verify
    assert_length(users, 2);
    mock_db.verify(); // Ensures all expectations were called
}
```

### Function Mocking

```rust
// Mock global functions
@test
fn time_dependent_test() {
    // Save original
    let original = get_current_time;
    
    // Mock function
    mock_function(get_current_time, || { 12345 });
    
    // Test code that uses get_current_time()
    let result = process_data();
    assert_eq(result.timestamp, 12345);
    
    // Restore
    restore_function(get_current_time);
}

// Or with scope:
@test
fn scoped_mock() {
    with_mock(get_current_time, || { 12345 }, || {
        // Mock is active here
        let result = process_data();
        assert_eq(result.timestamp, 12345);
    });
    // Mock is automatically restored
}
```

---

## 📦 Fixtures & Helpers

### Fixtures

```rust
// Define fixture
fixture "test database" -> Database {
    let db = Database::connect_test();
    db.migrate();
    db.seed_test_data();
    return db;
}

fixture "test sprite" -> Sprite {
    return Sprite {
        width: 32,
        height: 32,
        x: 0.0,
        y: 0.0,
    };
}

// Use fixture
@test
fn using_fixture() {
    let db = use_fixture("test database");
    let users = db.query("SELECT * FROM users");
    assert_length(users, 10); // From seed data
}

// Cleanup is automatic (Drop trait)
```

### Scoped Helpers

```rust
mod database_tests {
    // Helper functions scoped to this module
    fn create_test_user() -> User {
        User::new("test_user", "password")
    }
    
    fn assert_user_valid(user: &User) {
        assert(!user.username.is_empty());
        assert(user.password.len() >= 8);
    }
    
    @test
    fn user_creation() {
        let user = create_test_user();
        assert_user_valid(&user);
    }
    
    @test
    fn user_authentication() {
        let user = create_test_user();
        assert(user.verify_password("password"));
    }
}
```

---

## 🏃 Benchmarking

```rust
@bench
fn sprite_rendering_benchmark() {
    let sprite = Sprite::new();
    
    bench_iterations(1000, || {
        render_sprite(&sprite);
    });
    
    // Output: sprite_rendering_benchmark: 1.234ms per iteration
}

@bench
fn comparison_benchmark() {
    let old_algorithm = || { /* ... */ };
    let new_algorithm = || { /* ... */ };
    
    let old_time = bench(old_algorithm);
    let new_time = bench(new_algorithm);
    
    println!("Speedup: {}x", old_time / new_time);
}
```

---

## 📊 Test Output

### Standard Format

```
Running 15 tests...

✓ sprite_tests::creation (1ms)
✓ sprite_tests::animation (2ms)
✓ sprite_tests::collision (1ms)
✓ camera_tests::movement (1ms)
✓ camera_tests::zoom (1ms)
✗ physics_tests::collision_detection (3ms)
  
  Assertion failed at physics_tests.wj:45
  Expected: true
  Actual:   false
  
  assert(box1.collides_with(box2));
         ^^^^^^^^^^^^^^^^^^^^^

✓ ecs_tests::create_entity (0ms)
✓ ecs_tests::destroy_entity (1ms)

Test Results:
  Passed: 14/15 (93%)
  Failed: 1
  Ignored: 0
  Total time: 10ms
```

### Verbose Format

```
Running 15 tests...

[PASS] sprite_tests::creation (1ms)
  ↳ Sprite created with correct dimensions

[PASS] sprite_tests::animation (2ms)
  ↳ Animation advances frames correctly

[FAIL] physics_tests::collision_detection (3ms)
  ↳ Collision not detected between overlapping boxes
  
  Stack trace:
    at physics_tests::collision_detection (physics_tests.wj:45)
    at test_runner::run_test (test_runner.wj:123)
  
  assert(box1.collides_with(box2));
         ^^^^^^^^^^^^^^^^^^^^^
  
  Actual values:
    box1.position = Vec2(10.0, 10.0)
    box1.size = Vec2(5.0, 5.0)
    box2.position = Vec2(12.0, 12.0)
    box2.size = Vec2(5.0, 5.0)
```

---

## 🔨 Implementation Plan

### Phase 1: Parser & Syntax (Week 1)

**Goal:** Parse `#[test]` attribute and generate Rust test

```rust
// Lexer additions:
Token::Test
Token::TestCases
Token::PropertyTest
Token::Bench
Token::Requires
Token::Ensures
Token::Invariant

// AST additions:
TestDeclaration {
    name: String,
    attributes: Vec<Attribute>,
    setup: Option<String>,    // Function name
    teardown: Option<String>, // Function name
    body: Block,
}
```

### Phase 2: Code Generation (Week 1)

**Goal:** Compile Windjammer tests to Rust

```rust
// Windjammer:
@test
fn sprite_creation() {
    let sprite = Sprite::new();
    assert_eq(sprite.width, 32);
}

// Generates Rust:
#[test]
fn sprite_creation() {
    let sprite = Sprite::new();
    assert_eq!(sprite.width, 32);
}
```

### Phase 3: Assertion Library (Week 2)

**Goal:** Implement assertion functions in `windjammer-runtime`

```rust
// windjammer-runtime/src/test/assertions.rs
pub fn assert(condition: bool) {
    if !condition {
        panic!("Assertion failed");
    }
}

pub fn assert_eq<T: PartialEq + Debug>(left: T, right: T) {
    if left != right {
        panic!("Assertion failed:\n  left: {:?}\n right: {:?}", left, right);
    }
}

// ... etc
```

### Phase 4: Test Discovery (Week 2)

**Goal:** Automatically find and compile `.wj` test files

```rust
// In build.rs:
fn discover_tests() {
    let test_files = glob("tests/**/*.wj");
    
    for test_file in test_files {
        let rust_output = compile_windjammer_test(test_file);
        write_rust_test(rust_output);
    }
}
```

### Phase 5: Advanced Features (Week 3-4)

**Goal:** Mocking, fixtures, property tests, benchmarks

- Mocking system
- Fixture management
- Property-based testing (generators)
- Benchmark infrastructure
- Contract enforcement (requires/ensures)

---

## 📁 File Structure

```
windjammer-runtime/
  src/
    test/
      mod.rs              # Re-exports
      assertions.rs       # assert_eq, assert_ne, etc.
      mocking.rs          # Mock system
      fixtures.rs         # Fixture management
      generators.rs       # Property-based testing
      benchmarks.rs       # Performance testing
      contracts.rs        # Design-by-contract
      output.rs           # Test result formatting

windjammer-game/
  tests/
    sprite_tests.wj       -> compiles to -> build/tests/sprite_tests.rs
    camera_tests.wj       -> compiles to -> build/tests/camera_tests.rs
    ecs_tests.wj          -> compiles to -> build/tests/ecs_tests.rs
```

---

## 🎯 Success Criteria

### Week 1
- [x] `@test` decorator parses correctly
- [x] Simple test compiles to Rust
- [x] Basic assertions work (`assert`, `assert_eq`)

### Week 2
- [x] All basic assertion functions work
- [x] Test discovery works (finds `.wj` files)
- [x] `wj test` runs Windjammer tests

### Week 4
- [ ] Convert 10 existing tests to Windjammer
- [ ] Prove: tests are easier to write in Windjammer

### Week 8
- [ ] 50% of tests converted to Windjammer
- [ ] Mocking system functional

### Week 12
- [ ] 100% self-hosted (all tests in Windjammer)
- [ ] Property-based testing works
- [ ] Benchmark system functional

---

## 🔄 Migration Strategy

### Step 1: Build Foundation (Weeks 1-2)
- Implement core test framework
- Basic assertions
- Test discovery

### Step 2: Prove It Works (Week 3)
Convert 10 representative tests:
- 3 simple tests (sprite, camera)
- 3 parameterized tests (math operations)
- 2 tests with setup/teardown (database)
- 2 property tests (vector operations)

### Step 3: Gradual Migration (Weeks 4-8)
- Convert new tests in Windjammer
- Convert old tests as we touch them
- Keep Rust tests working during transition

### Step 4: Complete Self-Hosting (Weeks 8-12)
- Convert all remaining tests
- Remove Rust test files
- 100% Windjammer testing

---

## 🚀 Benefits

### 1. Dogfooding
- Every test exercises the Windjammer compiler
- Catches bugs in real usage
- Validates syntax design decisions

### 2. Developer Experience
- Tests feel like Windjammer code
- Less context switching (no Rust syntax)
- Cleaner, more readable

### 3. Self-Hosting
- Windjammer tests itself
- Reduces Rust dependency
- Proves language maturity

### 4. Competitive Advantage
- Unity: C# tests (separate from game code)
- Unreal: C++ tests (complex, verbose)
- Godot: GDScript tests (limited features)
- **Windjammer: Built-in, first-class testing**

---

## 📚 References

- **Rust:** `#[test]` attribute model
- **Go:** Table-driven tests
- **Haskell/QuickCheck:** Property-based testing
- **Eiffel:** Design-by-contract
- **Rust Doc Tests:** Inline examples

---

## 🎬 Next Steps

**After "The Unreal Look" is complete:**

1. **Week 1:** Implement parser + basic codegen
2. **Week 2:** Build assertion library + test discovery
3. **Week 3:** Convert 10 tests + prove it works
4. **Week 4:** Advanced features (mocking, fixtures)
5. **Week 8:** 50% tests converted
6. **Week 12:** 100% self-hosted

**Current Status:** Design phase (this document)  
**Blocked By:** Rendering features (Shadow Mapping, Post-Processing)  
**ETA:** Start in ~2-4 weeks

---

**Document Version:** 1.0  
**Last Updated:** 2026-01-05  
**Author:** Windjammer Core Team

