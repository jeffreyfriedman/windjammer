# std.testing - Test Framework

Integrated testing framework with assertions, benchmarking, and test organization.

## API

### Test Annotations

```windjammer
use std.testing

// Mark function as a test
@test
fn test_addition() {
    assert_eq(2 + 2, 4)
}

// Async test
@test
async fn test_async_operation() {
    let result = fetch_data().await
    assert(result.is_ok())
}

// Test that should panic
@test(should_panic)
fn test_panic() {
    panic("This should panic!")
}

// Test with custom message
@test(should_panic: "division by zero")
fn test_division_by_zero() {
    let _ = 1 / 0
}
```

### Assertions

```windjammer
// Basic assertions
fn assert(condition: bool)
fn assert_with_message(condition: bool, message: string)

// Equality
fn assert_eq<T>(left: T, right: T)
fn assert_ne<T>(left: T, right: T)

// Numeric comparisons
fn assert_gt(left: int, right: int)  // greater than
fn assert_lt(left: int, right: int)  // less than
fn assert_ge(left: int, right: int)  // greater or equal
fn assert_le(left: int, right: int)  // less or equal

// Option/Result assertions
fn assert_some<T>(option: Option<T>) -> T
fn assert_none<T>(option: Option<T>)
fn assert_ok<T, E>(result: Result<T, E>) -> T
fn assert_err<T, E>(result: Result<T, E>) -> E

// String assertions
fn assert_contains(haystack: string, needle: string)
fn assert_starts_with(text: string, prefix: string)
fn assert_ends_with(text: string, suffix: string)

// Collection assertions
fn assert_empty<T>(collection: Vec<T>)
fn assert_len<T>(collection: Vec<T>, expected: int)
```

### Test Organization

```windjammer
// Test module
mod tests {
    use super.*
    use std.testing
    
    @test
    fn test_feature_a() {
        // Test code
    }
    
    @test
    fn test_feature_b() {
        // Test code
    }
}

// Setup and teardown
@before_each
fn setup() {
    // Runs before each test
}

@after_each
fn teardown() {
    // Runs after each test
}
```

### Benchmarking

```windjammer
use std.testing

@bench
fn bench_algorithm() {
    // Code to benchmark
    let result = complex_calculation()
}

@bench(iterations: 1000)
fn bench_with_iterations() {
    // Run 1000 times
}
```

## Example Usage

### Basic Tests

```windjammer
use std.testing

@test
fn test_string_concatenation() {
    let result = "Hello, " + "World!"
    assert_eq(result, "Hello, World!")
}

@test
fn test_vector_operations() {
    let mut vec = vec![1, 2, 3]
    vec.push(4)
    
    assert_len(vec, 4)
    assert_eq(vec[3], 4)
}

@test
fn test_option_handling() {
    let some_value = Some(42)
    let result = assert_some(some_value)
    assert_eq(result, 42)
    
    let none_value: Option<int> = None
    assert_none(none_value)
}
```

### Testing with String Interpolation

```windjammer
@test
fn test_user_creation() {
    let user = User { name: "Alice", age: 30 }
    
    assert_eq(user.name, "Alice")
    assert_with_message(
        user.age >= 18,
        "User ${user.name} must be 18 or older, got ${user.age}"
    )
}
```

### Result Testing

```windjammer
@test
fn test_file_operations() {
    let result = fs.read_to_string("test.txt")
    
    match result {
        Ok(content) => {
            assert_contains(content, "expected text")
        }
        Err(e) => {
            panic("Failed to read file: ${e}")
        }
    }
}

// Or more concisely
@test
fn test_file_read() {
    let content = assert_ok(fs.read_to_string("test.txt"))
    assert_contains(content, "expected")
}
```

### Table-Driven Tests

```windjammer
@test
fn test_addition_cases() {
    let cases = [
        (2, 2, 4),
        (5, 3, 8),
        (0, 0, 0),
        (-1, 1, 0),
    ]
    
    for (a, b, expected) in cases {
        let result = add(a, b)
        assert_eq(result, expected)
    }
}
```

### Testing with Pipe Operator

```windjammer
@test
fn test_pipeline() {
    let result = "  hello world  "
        |> String.trim
        |> String.to_uppercase
    
    assert_eq(result, "HELLO WORLD")
}
```

## Running Tests

```bash
# Run all tests
wj test

# Run specific test
wj test test_addition

# Run tests with output
wj test --verbose

# Run benchmarks
wj bench
```

## Test Output

```
Running 5 tests...

test test_addition ... ✓ passed in 0.1ms
test test_string_concat ... ✓ passed in 0.2ms
test test_vector_ops ... ✓ passed in 0.3ms
test test_should_panic ... ✓ passed in 0.1ms
test test_async_op ... ✓ passed in 5.2ms

Results: 5 passed, 0 failed, 0 ignored

Total time: 5.9ms
```

## Features

✅ **Intuitive Assertions** - Clear, readable test code  
✅ **Rich Assertion Library** - Cover common test scenarios  
✅ **Async Test Support** - Test async functions naturally  
✅ **Benchmark Support** - Measure performance  
✅ **Organize with Modules** - Group related tests  
✅ **Setup/Teardown** - Before/after test hooks  
✅ **Custom Messages** - Helpful failure messages with interpolation  

---

**Status**: API Design Complete  
**Implementation**: Pending  
**Rust Deps**: Built-in test framework + custom macros

