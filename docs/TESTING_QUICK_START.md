# Windjammer Testing - Quick Start Guide

**Getting started with Windjammer's comprehensive testing framework in 5 minutes!**

---

## 📚 Table of Contents

1. [Your First Test](#your-first-test)
2. [Common Test Patterns](#common-test-patterns)
3. [Quick Reference](#quick-reference)
4. [Complete Examples](#complete-examples)

---

## Your First Test

Create a file `tests/my_test.wj`:

```rust
use std::test::*;

@test
fn my_first_test() {
    let result = 2 + 2;
    assert_eq(result, 4);
}
```

Run it:
```bash
wj test
```

**Congratulations! You just wrote your first Windjammer test!** 🎉

---

## Common Test Patterns

### ✅ Basic Assertions

```rust
@test
fn test_player_health() {
    let health = 100;
    
    assert_eq(health, 100);           // Equal
    assert_ne(health, 50);            // Not equal
    assert_gt(health, 50);            // Greater than
    assert_lt(health, 200);           // Less than
    assert_in_range(health, 0, 100);  // In range
}
```

### 📊 Parameterized Tests

```rust
@test_cases([
    (1, 1, 2),
    (2, 3, 5),
    (10, 20, 30),
])
fn test_addition(a: int, b: int, expected: int) {
    assert_eq(a + b, expected);
}
```

### ⏱️ Benchmarking

```rust
@test
fn test_performance() {
    let avg_time = bench_iterations(1000, || {
        render_frame();
    });
    
    println!("Average: {:?}", avg_time);
}
```

### 🎯 Property Testing

```rust
@test
fn test_addition_commutative() {
    for i in 0..100 {
        let a = i * 13;
        let b = i * 17;
        assert_eq(a + b, b + a);  // Property: commutative
    }
}
```

### 🔧 Fixtures

```rust
@test
fn test_with_database() {
    register_fixture("db", || Database::connect_test());
    
    let db = use_fixture::<Database>("db").unwrap();
    assert!(db.is_connected());
}
```

### 📝 Contracts

```rust
fn divide(a: int, b: int) -> int {
    requires(b != 0, "divisor must be non-zero");
    let result = a / b;
    ensures(result * b <= a, "division property");
    result
}
```

### 🎭 Mocking

```rust
@test
fn test_with_mock() {
    let tracker = MockTracker::new();
    tracker.record_call("save_game", vec![]);
    
    assert_eq(tracker.call_count("save_game"), 1);
    tracker.verify_called("save_game");
}
```

---

## Quick Reference

### Basic Assertions (20 functions)

| Function | Purpose | Example |
|----------|---------|---------|
| `assert_eq(a, b)` | Values equal | `assert_eq(x, 5)` |
| `assert_ne(a, b)` | Values not equal | `assert_ne(x, 0)` |
| `assert_gt(a, b)` | Greater than | `assert_gt(x, 0)` |
| `assert_lt(a, b)` | Less than | `assert_lt(x, 100)` |
| `assert_gte(a, b)` | Greater or equal | `assert_gte(x, 0)` |
| `assert_lte(a, b)` | Less or equal | `assert_lte(x, 100)` |
| `assert_approx(a, b, e)` | Floating point | `assert_approx(pi, 3.14, 0.01)` |
| `assert_contains(col, item)` | Collection contains | `assert_contains(&items, &5)` |
| `assert_empty(col)` | Collection empty | `assert_empty(&vec)` |
| `assert_not_empty(col)` | Collection not empty | `assert_not_empty(&vec)` |
| `assert_str_contains(s, sub)` | String contains | `assert_str_contains(text, "hello")` |
| `assert_starts_with(s, pre)` | String starts with | `assert_starts_with(text, "Hello")` |
| `assert_ends_with(s, suf)` | String ends with | `assert_ends_with(text, "world")` |
| `assert_is_some(opt)` | Option is Some | `assert_is_some(&option)` |
| `assert_is_none(opt)` | Option is None | `assert_is_none(&option)` |
| `assert_is_ok(res)` | Result is Ok | `assert_is_ok(&result)` |
| `assert_is_err(res)` | Result is Err | `assert_is_err(&result)` |
| `assert_in_range(v, min, max)` | Value in range | `assert_in_range(x, 0, 100)` |
| `assert_panics(fn)` | Code panics | `assert_panics(\|\| panic!())` |
| `assert_deep_eq(a, b)` | Deep equality | `assert_deep_eq(s1, s2)` |

### Test Decorators

| Decorator | Purpose | Example |
|-----------|---------|---------|
| `@test` | Mark as test | `@test fn my_test() {}` |
| `@ignore` | Skip test | `@test @ignore fn skip_me() {}` |
| `@test_cases([...])` | Parameterized | `@test_cases([(1,2)]) fn test(a,b) {}` |

### Benchmarking (3 functions)

| Function | Purpose | Example |
|----------|---------|---------|
| `bench(fn)` | Single run | `bench(\|\| do_work())` |
| `bench_iterations(n, fn)` | Average over n | `bench_iterations(1000, \|\| render())` |
| `bench_compare(f1, f2, n)` | Compare 2 functions | `bench_compare(old, new, 100)` |

### Property Testing (5 functions)

| Function | Purpose |
|----------|---------|
| `property_test(n, prop)` | Test property n times |
| `with_gen(gen, prop)` | Test with generator |
| `with_gen2(g1, g2, prop)` | Test with 2 generators |
| `with_gen3(g1, g2, g3, prop)` | Test with 3 generators |
| `find_minimal_failing(fn)` | Find smallest failure |

### Mocking (3 types)

| Type | Purpose | Example |
|------|---------|---------|
| `MockTracker` | Track calls | `tracker.record_call("fn", vec![])` |
| `MockReturn<T>` | Return values | `mock.next()` |
| `MockObject` | Full mocking | `mock.expect(Expectation::new("fn"))` |

### Contracts (4 functions)

| Function | Purpose | Example |
|----------|---------|---------|
| `requires(cond, msg)` | Precondition | `requires(x > 0, "positive")` |
| `ensures(cond, msg)` | Postcondition | `ensures(result > 0, "positive")` |
| `invariant(cond, msg)` | Invariant | `invariant(count >= 0, "valid")` |
| `old(value)` | Capture pre-state | `let old_x = old(x)` |

### Fixtures (3 functions)

| Function | Purpose | Example |
|----------|---------|---------|
| `register_fixture(name, fn)` | Register | `register_fixture("db", \|\| DB::new())` |
| `use_fixture<T>(name)` | Use | `use_fixture::<DB>("db")` |
| `FixtureScope::new(val)` | RAII cleanup | `FixtureScope::new(resource)` |

### Lifecycle (3 functions)

| Function | Purpose |
|----------|---------|
| `with_setup(setup, test)` | Setup only |
| `with_teardown(teardown, test)` | Teardown only |
| `with_setup_teardown(s, t, test)` | Full lifecycle |

### Timeout (1 function)

| Function | Purpose | Example |
|----------|---------|---------|
| `with_timeout(dur, fn)` | Time limit | `with_timeout(Duration::from_secs(1), \|\| test())` |

---

## Complete Examples

See `examples/testing_examples.wj` for 50+ complete, copy-paste ready examples covering:

1. ✅ Basic assertions
2. ✅ Advanced assertions
3. ✅ Parameterized tests
4. ✅ Skipping tests
5. ✅ Benchmarking
6. ✅ Property testing
7. ✅ Test output
8. ✅ Timeout
9. ✅ Setup/Teardown
10. ✅ Fixtures
11. ✅ Doc tests
12. ✅ Contracts
13. ✅ Basic mocking
14. ✅ Interface mocking
15. ✅ Function mocking
16. ✅ Complete game example

---

## Common Workflows

### 🎮 Game Testing Workflow

```rust
// 1. Test game logic with contracts
fn take_damage(health: int, damage: int) -> int {
    requires(damage >= 0, "damage non-negative");
    let result = health - damage;
    if result < 0 { 0 } else { result }
}

// 2. Parameterized tests for coverage
@test_cases([
    (100, 10, 90),
    (100, 150, 0),
])
fn test_damage(h: int, d: int, exp: int) {
    assert_eq(take_damage(h, d), exp);
}

// 3. Property tests for invariants
@test
fn test_health_never_negative() {
    for i in 0..100 {
        let damage = i * 13;
        let result = take_damage(100, damage);
        assert_gte(result, 0);
    }
}

// 4. Benchmark performance
@test
fn test_performance() {
    let time = bench_iterations(1000, || {
        take_damage(100, 10);
    });
    assert!(time < Duration::from_micros(10));
}
```

### 🚀 Performance Testing Workflow

```rust
@test
fn ensure_60fps() {
    // Frame budget: 16.67ms for 60fps
    let result = with_timeout(
        Duration::from_millis(16),
        || render_frame()
    );
    assert!(result.is_ok());
}

@test
fn benchmark_improvements() {
    let (old, new, speedup) = bench_compare(
        || old_algorithm(),
        || new_algorithm(),
        100
    );
    
    assert!(speedup > 1.5);  // At least 1.5x faster
}
```

### 🔧 Integration Testing Workflow

```rust
@test
fn test_game_flow() {
    // Setup
    register_fixture("level", || Level::load_test());
    register_fixture("player", || Player::new());
    
    // Test
    with_setup_teardown(
        || GameState::new(),
        |state| state.cleanup(),
        |state| {
            let level = use_fixture::<Level>("level").unwrap();
            let player = use_fixture::<Player>("player").unwrap();
            
            state.start(level, player);
            assert(state.is_running());
            
            state
        }
    );
}
```

---

## Tips & Best Practices

### ✅ DO:
- **Write tests first** (TDD)
- **Use descriptive test names** (`test_player_takes_damage_when_hit`)
- **Test one thing per test** (focused tests)
- **Use parameterized tests** for multiple inputs
- **Use property tests** for invariants
- **Benchmark critical paths** (rendering, physics)
- **Use contracts** for critical functions
- **Mock external dependencies** (network, files)

### ❌ DON'T:
- **Don't test implementation details** (test behavior)
- **Don't write flaky tests** (avoid timing-dependent tests)
- **Don't ignore failing tests** (fix or remove)
- **Don't skip benchmarks** (performance matters!)
- **Don't forget to test edge cases** (0, negative, max values)

---

## Next Steps

1. **Read examples:** `examples/testing_examples.wj` (795 lines of examples!)
2. **Read design doc:** `docs/WINDJAMMER_TESTING_FRAMEWORK.md` (complete reference)
3. **Write your first test** and run `wj test`
4. **Start with simple assertions** and gradually add advanced features
5. **Use TDD:** Write tests before implementation

---

## Need Help?

- **Examples:** `examples/testing_examples.wj`
- **Full docs:** `docs/WINDJAMMER_TESTING_FRAMEWORK.md`
- **Validation:** `tests/framework_validation.wj`

---

**Happy Testing!** 🚀

*The Windjammer Testing Framework makes testing fun and productive!*


