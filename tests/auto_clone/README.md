# Auto-Clone Test Suite

Comprehensive tests for Windjammer's automatic clone insertion system.

## Test Coverage

### 1. Simple Variables (`test_simple_variables.wj`)
- ✅ Vec auto-clone
- ✅ String auto-clone
- ✅ Multiple uses of same variable

### 2. Field Access (`test_field_access.wj`)
- ✅ Struct field access (`config.paths`)
- ✅ Nested field access
- ✅ Multiple uses of same field

### 3. Method Calls (`test_method_calls.wj`)
- ✅ Method call results (`source.get_items()`)
- ✅ Chained method calls
- ✅ Multiple uses of same method result

### 4. Index Expressions (`test_index_expressions.wj`)
- ✅ Literal index (`items[0]`)
- ✅ Variable index (`items[idx]`)
- ✅ Multiple uses of same index

### 5. Combined Patterns (`test_combined_patterns.wj`)
- ✅ Field then method (`obj.field.method()`)
- ✅ Method then index (`obj.method()[0]`)
- ✅ Complex combinations

## Running Tests

```bash
# Run all tests
./run_tests.sh

# Run individual test
wj build test_simple_variables.wj -o /tmp/test && cd /tmp/test && cargo run

# Run with verbose output
./run_tests.sh --verbose
```

## Expected Behavior

All tests should:
1. **Compile successfully** - No manual `.clone()` calls needed
2. **Run without errors** - All assertions pass
3. **Demonstrate auto-clone** - Values usable after being moved

## What Auto-Clone Does

The compiler automatically inserts `.clone()` calls when:
1. A value is moved to a function
2. The value is used again later in the same scope

Example:
```windjammer
// User writes:
let data = vec![1, 2, 3]
process(data)
println!("{}", data.len())

// Compiler generates:
let data = vec![1, 2, 3]
process(data.clone())  // Auto-inserted!
println!("{}", data.len())
```

## Success Criteria

✅ All 5 test files compile without errors
✅ All 5 test files run successfully
✅ Zero manual `.clone()` calls in test code
✅ All assertions pass

## Philosophy

These tests demonstrate Windjammer's core promise:
**"Write simple code, compiler handles complexity"**

Users never need to think about ownership, borrowing, or lifetimes.
The compiler automatically manages memory safety while maintaining ergonomics.

