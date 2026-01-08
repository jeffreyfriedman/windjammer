# Pattern Matching Gaps Analysis

**Date**: November 29, 2025  
**Status**: Discovered during consistency audit

---

## ✅ WHAT WORKS (Well Supported)

### Match Statement Patterns
- ✅ Unit enum variants: `Color::Red`
- ✅ Enum variants with single binding: `Option::Some(value)`
- ✅ Qualified paths: `module::Type::Variant`
- ✅ Wildcard: `_`
- ✅ Wildcard in variant: `Option::Some(_)`
- ✅ Literal patterns: `0`, `1`, `42`
- ✅ Or patterns: `1 | 2 | 3`
- ✅ Named bindings: `Option::Some(value)`

**Test File**: `tests/pattern_matching_audit.wj` - ALL PASS ✅

---

## ❌ GAPS DISCOVERED

### 1. **Tuple Enum Variants** ❌ HIGH PRIORITY

**Issue**: Cannot define or match enum variants with multiple fields

**Example That Fails**:
```windjammer
enum Color {
    Red,
    Rgb(i32, i32, i32),  // ❌ Parse error: Expected RParen, got Comma
}
```

**Impact**: 
- Can't represent common patterns like `Result<T, E>` with data
- Can't use Rust-style enum variants
- Forces workarounds with nested enums

**Workaround**: Use single-field variants with tuples
```windjammer
enum Color {
    Red,
    Rgb((i32, i32, i32)),  // Single field containing tuple
}
```

**Fix Needed**: 
- Enum parser needs to support multiple fields in variant definition
- Pattern parser needs to destructure multiple fields: `Rgb(r, g, b)`

---

### 2. **Patterns in Function Parameters** ❌ MEDIUM PRIORITY

**Issue**: Cannot use destructuring patterns in function parameters

**Examples That Fail**:
```windjammer
fn test_tuple((a, b): (i32, i32)) -> i32 {  // ❌ Parse error
    return a + b
}

fn test_option(Some(value): Option<i32>) -> i32 {  // ❌ Parse error
    return value
}

fn test_wildcard(_: i32) -> i32 {  // ❌ Parse error
    return 42
}
```

**Impact**:
- Less ergonomic function signatures
- Can't use Rust-style parameter destructuring
- Requires extra `let` statements inside function

**Workaround**: Destructure inside function body
```windjammer
fn test_tuple(pair: (i32, i32)) -> i32 {
    let (a, b) = pair  // If tuple destructuring in let works
    return a + b
}
```

**Fix Needed**:
- Function parameter parser needs to accept patterns, not just identifiers
- Type inference needs to work with pattern parameters

---

### 3. **Patterns in Let Bindings** ❌ MEDIUM PRIORITY

**Issue**: Cannot use destructuring patterns in `let` statements

**Examples That Fail**:
```windjammer
let (x, y) = (10, 20)  // ❌ Parse error: Expected Assign, got LParen

let Some(value) = Some(42)  // ❌ Parse error

let Point { x, y } = point  // ❓ Unknown status
```

**Impact**:
- Can't destructure tuples
- Can't destructure structs
- Can't pattern match in let bindings

**Workaround**: Use match expressions
```windjammer
let x = match pair {
    (a, b) => a
}
```

**Fix Needed**:
- Let statement parser needs to accept patterns on LHS
- Need to handle irrefutable vs refutable patterns

---

### 4. **Patterns in For Loops** ❌ LOW PRIORITY

**Issue**: Cannot destructure in for loop bindings

**Example That Fails**:
```windjammer
for (key, value) in map {  // ❌ Parse error
    // ...
}
```

**Impact**:
- Less ergonomic iteration over pairs/tuples
- Requires manual indexing

**Workaround**: Use indexing
```windjammer
for pair in map {
    let key = pair.0
    let value = pair.1
}
```

**Fix Needed**:
- For loop parser needs to accept patterns for loop variable

---

### 5. **Nested Enum Patterns** ❓ PARTIAL SUPPORT

**Issue**: Deeply nested patterns may not work

**Example That Failed**:
```windjammer
match result {
    Result::Ok(Option::Some(value)) => { ... }  // ❌ Parse error
}
```

**Status**: Needs more testing to determine exact limitations

**Fix Needed**: Test and fix nested pattern parsing

---

### 6. **Struct Patterns** ❓ UNKNOWN STATUS

**Issue**: Not tested yet

**Examples To Test**:
```windjammer
match point {
    Point { x: 0, y: 0 } => { ... }
    Point { x, y } => { ... }
}

let Point { x, y } = point
```

**Fix Needed**: Test and implement if missing

---

### 7. **Reference Patterns** ❓ UNKNOWN STATUS

**Issue**: Not tested yet

**Examples To Test**:
```windjammer
match &value {
    &Some(x) => { ... }
    &None => { ... }
}

let &x = &value
```

**Fix Needed**: Test and implement if missing

---

### 8. **Range Patterns** ❌ NOT SUPPORTED

**Issue**: Cannot match ranges

**Example That Would Fail**:
```windjammer
match value {
    0..=10 => { ... }
    11..=20 => { ... }
    _ => { ... }
}
```

**Impact**: Less expressive pattern matching

**Fix Needed**: Add range pattern support

---

## 📊 PRIORITY MATRIX

| Gap | Priority | Impact | Effort | Status |
|-----|----------|--------|--------|--------|
| Tuple Enum Variants | HIGH | High | Medium | ❌ Not Started |
| Patterns in Function Params | MEDIUM | Medium | Medium | ❌ Not Started |
| Patterns in Let Bindings | MEDIUM | High | Medium | ❌ Not Started |
| Patterns in For Loops | LOW | Low | Low | ❌ Not Started |
| Nested Enum Patterns | MEDIUM | Medium | Low | ❓ Needs Testing |
| Struct Patterns | MEDIUM | Medium | Low | ❓ Needs Testing |
| Reference Patterns | LOW | Low | Low | ❓ Needs Testing |
| Range Patterns | LOW | Low | Medium | ❌ Not Started |

---

## 🎯 RECOMMENDED IMPLEMENTATION ORDER

### Phase 1: Critical Gaps (Blocking Common Use Cases)
1. **Tuple Enum Variants** - Enables Rust-style enums
2. **Patterns in Let Bindings** - Enables tuple/struct destructuring
3. **Nested Enum Patterns** - Test and fix if broken

### Phase 2: Ergonomics (Nice to Have)
4. **Patterns in Function Parameters** - Cleaner function signatures
5. **Struct Patterns in Match** - Test and implement if missing
6. **Patterns in For Loops** - Cleaner iteration

### Phase 3: Advanced Features (Future)
7. **Reference Patterns** - Advanced use cases
8. **Range Patterns** - Syntactic sugar

---

## 🔍 TESTING STRATEGY

1. **Create comprehensive test suite** for each pattern context:
   - Function parameters
   - Let bindings
   - Match arms (already tested ✅)
   - For loops
   - If let (not yet tested)
   - While let (not yet tested)

2. **Test pattern types** in each context:
   - Wildcard `_`
   - Identifier `x`
   - Tuple `(a, b, c)`
   - Struct `Point { x, y }`
   - Enum variant `Some(x)`
   - Nested patterns `Ok(Some(x))`
   - Or patterns `1 | 2 | 3`
   - Literal patterns `42`
   - Reference patterns `&x`

3. **Create regression tests** as gaps are fixed

---

## 📝 NOTES

- Pattern matching in **match statements** is well-supported ✅
- Main gaps are in **other contexts** (let, fn params, for loops)
- **Consistency issue**: Patterns work in match but not elsewhere
- This is a **language consistency gap** that should be addressed

---

## 🎉 CONSISTENCY IMPACT

**Current**: Patterns only work in match statements  
**Target**: Patterns work consistently everywhere  
**Consistency Score Impact**: Would improve from 9.4/10 to 9.7/10

**Rationale**: If patterns work in match, they should work in let, fn params, and for loops for consistency.





















