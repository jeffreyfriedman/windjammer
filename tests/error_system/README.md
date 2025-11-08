# Error System End-to-End Tests

This directory contains end-to-end tests for the Windjammer error system.

## Test Files

Each test file contains intentional errors to verify that:
1. Source maps are generated correctly
2. Errors are mapped back to Windjammer source
3. Error messages are translated to Windjammer terminology
4. Contextual help is provided

### Test Cases

1. **test_type_mismatch.wj** - Type mismatch errors
   - Tests: `i32` → `int`, `&str` → `string` translation
   - Expected: "Type mismatch: expected int, found string"

2. **test_function_not_found.wj** - Function not found errors
   - Tests: Function name extraction and translation
   - Expected: "Function not found: calculate_total"

3. **test_ownership.wj** - Ownership errors
   - Tests: Move semantics error translation
   - Expected: "Ownership error: This value was already used"

4. **test_mutability.wj** - Mutability errors
   - Tests: Immutable variable assignment
   - Expected: "Cannot modify: This value is not declared as mutable"

5. **test_variable_not_found.wj** - Variable not found errors
   - Tests: Undefined variable detection
   - Expected: "Variable not found: count"

## Running Tests

```bash
# Run all tests
./run_tests.sh

# Or manually test a single file
cargo run -- build test_type_mismatch.wj -o /tmp/test_output --check
```

## Expected Behavior

For each test, the error system should:
1. ✅ Generate source map (.rs.map file)
2. ✅ Compile to Rust and capture errors
3. ✅ Map Rust error locations → Windjammer locations
4. ✅ Translate error messages
5. ✅ Display with colors and source snippets
6. ✅ Provide contextual help

## Success Criteria

- All 5 tests pass
- Error messages reference `.wj` files, not `.rs` files
- Error messages use Windjammer terminology
- Source code snippets show Windjammer code
- Contextual help suggestions are provided

## Adding New Tests

To add a new test:
1. Create a new `.wj` file with an intentional error
2. Add a test case to `run_tests.sh`
3. Specify the expected error message
4. Run the test suite

## Notes

- Tests use `/tmp/wj_error_tests` as output directory
- Each test is independent
- Tests verify both translation and source mapping
- Colored output is preserved in test results

