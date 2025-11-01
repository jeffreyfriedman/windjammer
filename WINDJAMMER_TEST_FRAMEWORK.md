# Windjammer Test Framework Design

## Overview
A comprehensive testing framework for Windjammer, inspired by Rust's `cargo test` and Go's `go test`.

## Command: `wj test`

### Usage
```bash
# Run all tests in current directory
wj test

# Run tests in specific directory
wj test tests/

# Run specific test file
wj test tests/stdlib_test.wj

# Run tests matching pattern
wj test --filter http

# Show output from passing tests
wj test --nocapture

# Run tests sequentially
wj test --parallel=false
```

## Test Discovery

### Convention
- Test files: `*_test.wj` or `test_*.wj`
- Test functions: Start with `test_` prefix
- Located in:
  - `tests/` directory
  - Inline in source files

### Example Test File
```windjammer
// tests/stdlib_http_test.wj
use std::test
use std::http

fn test_server_response_ok() {
    let response = http::ServerResponse::ok("Hello")
    test::assert_eq(response.status, 200)
    test::assert_eq(response.body, "Hello")
}

fn test_server_response_not_found() {
    let response = http::ServerResponse::not_found()
    test::assert_eq(response.status, 404)
}
```

## Test API (`std::test`)

### Assertions
```windjammer
test::assert(condition, "message")
test::assert_eq(left, right, "message")
test::assert_ne(left, right, "message")
```

### Test Control
```windjammer
test::fail("reason")  // Explicitly fail a test
test::should_panic(fn() { ... })  // Expect panic
```

## Implementation Plan

### Phase 1: Test Discovery
1. Find all `*_test.wj` files
2. Parse each file to find `test_*` functions
3. Build list of test cases

### Phase 2: Test Compilation
1. Generate Rust test harness
2. Each Windjammer test → Rust `#[test]` function
3. Compile with `cargo test`

### Phase 3: Test Execution
1. Run `cargo test` with appropriate flags
2. Parse output
3. Display results in Windjammer format

### Phase 4: Reporting
1. Summary: X passed, Y failed
2. Failed test details
3. Timing information

## Output Format

```
Running 5 tests from 2 files

tests/stdlib_http_test.wj:
  test_server_response_ok ... ok
  test_server_response_not_found ... ok
  test_custom_status ... ok

tests/stdlib_math_test.wj:
  test_abs ... ok
  test_min_max ... FAILED

Failures:

---- test_min_max ----
Assertion failed: expected 5, got 10
  at tests/stdlib_math_test.wj:15

Test result: 4 passed, 1 failed
```

## Benefits
1. **Familiar**: Like Rust/Go test frameworks
2. **Integrated**: Part of `wj` CLI
3. **Fast**: Compiles to native Rust tests
4. **Discoverable**: Convention-based
5. **Self-testing**: Windjammer tests Windjammer!


