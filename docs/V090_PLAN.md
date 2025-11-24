# Windjammer v0.9.0 Development Plan

## 🎯 Theme: Enhanced Features & Polish

**Goal**: Implement generic trait implementations, enhance error mapping, and expand stdlib with essential modules.

**Timeline**: 1-2 weeks  
**Branch**: `feature/v0.9.0-enhanced-features`

---

## 📋 Features Overview

| Feature | Priority | Complexity | Status |
|---------|----------|------------|--------|
| Generic Trait Implementations | High | High | 🔜 Planned |
| Error Mapping Phase 2 | Medium | Medium | 🔜 Planned |
| Stdlib Collections Module | High | Medium | 🔜 Planned |
| Stdlib Testing Framework | Medium | Medium | 🔜 Planned |
| Generic Enums | Medium | Medium | 🔜 Planned |
| Additional Examples | Low | Low | 🔜 Planned |

---

## 🏗️ Phase 1: Generic Trait Implementations

### Goal
Enable implementing generic traits for types: `impl Trait<T> for Type`

### Current Limitation
- ✅ Can define generic traits: `trait From<T> { ... }`
- ❌ Cannot implement them: `impl From<int> for String { ... }`

### Syntax to Support

```windjammer
// Single type parameter
impl From<int> for String {
    fn from(value: int) -> Self {
        value.to_string()
    }
}

// Multiple type parameters
impl Converter<int, string> for IntToString {
    fn convert(&self, input: int) -> string {
        input.to_string()
    }
}

// Generic impl with bounds
impl<T: Display> From<T> for String {
    fn from(value: T) -> Self {
        format!("{}", value)
    }
}
```

### Implementation Tasks

1. **Parser Updates** (`src/parser.rs`):
   - [ ] Update `parse_impl()` to handle `<Type>` after trait name
   - [ ] Parse `impl Trait<T, U> for Type` syntax
   - [ ] Handle multiple type arguments
   - [ ] Parse generic impl blocks: `impl<T: Bound> Trait<T> for Type`

2. **AST Extensions**:
   - [ ] Add `trait_type_args: Option<Vec<Type>>` to `ImplBlock`
   - [ ] Distinguish between `impl<T>` (generic impl) and `Trait<T>` (trait args)

3. **Codegen** (`src/codegen.rs`):
   - [ ] Generate `impl Trait<Type> for Target { ... }`
   - [ ] Handle generic type arguments correctly
   - [ ] Generate bounds on generic impls

4. **Testing**:
   - [ ] Unit tests for parser
   - [ ] Example: `From<T>` implementations
   - [ ] Example: `Converter<Input, Output>` implementations

**Files to Modify**:
- `src/parser.rs`: Parse generic trait impl syntax
- `src/codegen.rs`: Generate Rust generic trait impls
- `examples/30_generic_trait_impls/main.wj`: New example

---

## 🏗️ Phase 2: Error Mapping Phase 2

### Goal
Enhanced error messages with pattern detection and contextual suggestions.

### Features

1. **Common Error Patterns**:
   ```
   Pattern: "mismatched types: expected &str, found &String"
   Suggestion: "In Windjammer, use &string which maps to &str in Rust"
   
   Pattern: "trait bound not satisfied: T: Display"
   Suggestion: "Add trait bound: fn func<T: Display>(x: T)"
   
   Pattern: "cannot move out of borrowed content"
   Suggestion: "Consider using .clone() or change ownership hints"
   ```

2. **Contextual Help**:
   - Link to relevant GUIDE.md sections
   - Provide Windjammer-specific fixes
   - Show before/after examples

3. **Implementation**:
   - [ ] Create `ErrorPattern` struct in `src/error_mapper.rs`
   - [ ] Build pattern database with regex matchers
   - [ ] Implement `suggest_fix()` function
   - [ ] Add colored output for suggestions
   - [ ] Generate documentation links

**Files to Modify**:
- `src/error_mapper.rs`: Pattern database and suggestions
- `src/main.rs`: Enhanced error translation
- `docs/ERROR_MAPPING.md`: Phase 2 documentation

---

## 🏗️ Phase 3: Stdlib Expansion

### 3.1 Collections Module (`std/collections.wj`)

Wrapper for Rust's `std::collections` with Windjammer-friendly APIs:

```windjammer
// HashMap
use std::collections

fn main() {
    let mut map = HashMap::new()
    map.insert("key", "value")
    
    match map.get("key") {
        Some(val) => println!("Found: {}", val),
        None => println!("Not found")
    }
}
```

**Functions to wrap**:
- `HashMap<K, V>`: new, insert, get, remove, contains_key, keys, values
- `HashSet<T>`: new, insert, remove, contains, len
- `BTreeMap<K, V>`: Similar to HashMap, sorted
- `BTreeSet<T>`: Similar to HashSet, sorted
- `VecDeque<T>`: Double-ended queue

**Implementation**:
- [ ] Create `std/collections.wj`
- [ ] Wrap HashMap operations
- [ ] Wrap HashSet operations
- [ ] Wrap BTreeMap/BTreeSet (optional)
- [ ] Add to `Cargo.toml` dependencies
- [ ] Create example: `examples/31_collections/main.wj`

### 3.2 Testing Framework (`std/testing.wj`)

Simple testing utilities:

```windjammer
use std::testing

@test
fn test_addition() {
    assert_eq(2 + 2, 4)
    assert_ne(2 + 2, 5)
}

@test
fn test_strings() {
    let s = "hello"
    assert(s.len() == 5)
}
```

**Functions to provide**:
- `assert(condition)`: Basic assertion
- `assert_eq(left, right)`: Equality assertion
- `assert_ne(left, right)`: Inequality assertion
- `fail(message)`: Explicit test failure

**Implementation**:
- [ ] Create `std/testing.wj`
- [ ] Implement assertion macros
- [ ] Test framework integration
- [ ] Create example: `examples/32_testing/main.wj`

---

## 🏗️ Phase 4: Generic Enums

### Goal
Support generic type parameters on enums.

### Syntax

```windjammer
// Basic generic enum
enum Container<T> {
    Value(T),
    Empty
}

// Multiple type parameters
enum Either<L, R> {
    Left(L),
    Right(R)
}

// With trait bounds
enum Wrapper<T: Display> {
    Some(T),
    None
}
```

### Implementation

1. **Parser Updates**:
   - [ ] Add `type_params: Vec<TypeParam>` to `EnumDecl`
   - [ ] Parse `<T, U>` after enum name
   - [ ] Parse bounds on enum type parameters

2. **Codegen Updates**:
   - [ ] Generate `enum Name<T> { ... }`
   - [ ] Handle generic type parameters

3. **Testing**:
   - [ ] Unit tests
   - [ ] Example: Generic container enums

**Files to Modify**:
- `src/parser.rs`: Parse generic enum syntax
- `src/codegen.rs`: Generate generic enums
- `examples/33_generic_enums/main.wj`: New example

---

## 📚 Examples to Create

1. **Example 30**: Generic Trait Implementations
   - `impl From<int> for String`
   - `impl<T: Display> Into<String> for T`
   - Demonstrates all variants

2. **Example 31**: Collections
   - HashMap usage
   - HashSet usage
   - Real-world data structure examples

3. **Example 32**: Testing Framework
   - Unit tests with assertions
   - Test organization
   - Running tests

4. **Example 33**: Generic Enums
   - `enum Option<T>`
   - `enum Result<T, E>`
   - Pattern matching with generics

---

## 📝 Documentation Updates

### Files to Update

1. **CHANGELOG.md**:
   - [ ] Add v0.9.0 entry
   - [ ] Document all new features
   - [ ] List examples

2. **README.md**:
   - [ ] Add generic trait impls section
   - [ ] Update feature list
   - [ ] Add stdlib modules

3. **docs/GUIDE.md**:
   - [ ] Generic trait implementations section
   - [ ] Collections usage guide
   - [ ] Testing framework guide
   - [ ] Generic enums section

4. **std/README.md**:
   - [ ] Document collections module
   - [ ] Document testing module
   - [ ] Update module list

---

## ✅ Success Criteria

### Must Have (Release Blockers)
- [ ] Generic trait implementations working
- [ ] Collections module functional
- [ ] All tests passing (100%)
- [ ] Zero clippy warnings
- [ ] Documentation updated
- [ ] At least 4 new examples (30-33)

### Should Have (Nice to Have)
- [ ] Error mapping Phase 2 complete
- [ ] Testing framework functional
- [ ] Generic enums working
- [ ] Performance benchmarks updated

### Could Have (Future)
- [ ] Additional stdlib modules
- [ ] More error patterns
- [ ] Enhanced testing utilities

---

## 🧪 Testing Strategy

### Unit Tests
- [ ] Parser tests for generic trait impls
- [ ] Parser tests for generic enums
- [ ] Codegen tests for all new features

### Integration Tests
- [ ] Generic trait impl examples compile
- [ ] Collections module works
- [ ] Testing framework works
- [ ] Generic enums compile

### Manual Testing
- [ ] All examples (30-33) compile and run
- [ ] Generated Rust code is idiomatic
- [ ] Error messages are helpful

---

## 📊 Development Phases

### Week 1: Core Features
**Days 1-2**: Generic Trait Implementations
- Parser updates
- Codegen implementation
- Testing and examples

**Days 3-4**: Generic Enums
- Parser updates
- Codegen implementation
- Testing and examples

**Days 5-7**: Stdlib Modules
- Collections module
- Testing framework
- Examples and documentation

### Week 2: Polish & Documentation
**Days 1-2**: Error Mapping Phase 2
- Pattern database
- Suggestion engine
- Testing

**Days 3-4**: Documentation
- Update all docs
- Write comprehensive examples
- GUIDE.md sections

**Days 5-7**: Testing & Release Prep
- All tests passing
- Clippy clean
- Final polish
- PR preparation

---

## 🚀 Release Checklist

Before merging to main:

- [ ] All features implemented
- [ ] All tests passing (100%)
- [ ] Zero clippy warnings
- [ ] `cargo fmt --all` clean
- [ ] All documentation updated
- [ ] CHANGELOG.md complete
- [ ] All examples working (30-33)
- [ ] PR comment prepared
- [ ] Release notes written

---

## 📚 References

- Rust Book: Chapter 10 (Generic Types, Traits, Lifetimes)
- Rust RFC 2027: Generic Associated Types
- Previous PRs: v0.8.0 (trait system)
- `docs/TRAIT_BOUNDS_DESIGN.md` - Design philosophy

---

## 🔮 Looking Ahead to v0.10.0

**New Strategic Direction: Inference Over Explicit Annotation**

Based on research in `docs/INFERENCE_DESIGN.md`, v0.10.0 will focus on achieving **"80% simplicity through 80% inference"** rather than copying Rust features verbatim.

### Core Features for v0.10.0:
- **Inferred Trait Bounds** (FLAGSHIP): Analyze usage to infer `T: Display`, `T: Clone`, etc.
- **Named Bound Sets**: `bound Printable = Display + Debug` for reusable constraints
- **Smart Decorators**: `@test`, `@async` that expand to full boilerplate
- **Enhanced stdlib** with pipeline-friendly methods

### Philosophy:
- Not "simpler than Rust" - "less repetitive than Rust"
- Progressive disclosure: implicit by default, explicit when needed
- Escape hatch: use explicit bounds or raw Rust for edge cases
- 80% of developers never write trait bounds, 20% use advanced features

See `docs/INFERENCE_DESIGN.md` for full research and implementation plan.

---

**Status**: Planning Complete ✅  
**Next Step**: Begin Phase 1 - Generic Trait Implementations  
**Branch**: `feature/v0.9.0-enhanced-features`  
**Target Release**: 1-2 weeks from start

**Remember**: v0.10.0 next, then v0.11.0, etc. - NOT v1.0.0 until production confidence!

