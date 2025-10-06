# Windjammer v0.8.0 Development Plan

## 🎯 Theme: Advanced Type System

**Goal**: Complete the trait system to enable advanced Rust patterns while maintaining Windjammer's simplicity (80/20 rule).

**Timeline**: 2-3 weeks  
**Branch**: `feature/v0.8.0-trait-system`

## 📋 Features Overview

| Feature | Priority | Complexity | Status |
|---------|----------|------------|--------|
| Trait Bounds (Level 1) | High | Medium | 🔜 Planned |
| Trait Bounds (Level 2) | High | Medium | 🔜 Planned |
| Where Clauses | Medium | Medium | 🔜 Planned |
| Associated Types | Medium | High | 🔜 Planned |
| Error Mapping Phase 2 | Low | Medium | 🔜 Planned |
| Stdlib Expansion | Low | Low | 🔜 Planned |

## 🏗️ Phase 1: Trait Bounds (Level 1 & 2)

### Level 1: Basic Inline Bounds

**Goal**: Support simple trait bounds on generic parameters.

**Syntax**:
```windjammer
// Function with bounded type parameter
fn print_all<T: Display>(items: Vec<T>) {
    for item in items {
        println!("${item}")
    }
}

// Struct with bounded type parameter
struct Container<T: Clone> {
    value: T
}

// Multiple bounds with +
fn compare<T: PartialEq + Debug>(a: T, b: T) -> bool {
    a == b
}
```

**Implementation Tasks**:

1. **Parser Updates** (`src/parser.rs`):
   - [x] Already have `TypeParam` struct with `bounds: Vec<String>`
   - [ ] Update `parse_type_params()` to parse `: Trait` syntax
   - [ ] Parse `+` for multiple bounds (e.g., `T: Display + Clone`)
   - [ ] Validate bound syntax and error messages

2. **AST Validation** (`src/analyzer.rs`):
   - [ ] Validate that bounds reference valid traits
   - [ ] Check that bounded types are used correctly
   - [ ] Add test cases for invalid bound usage

3. **Codegen** (`src/codegen.rs`):
   - [ ] Generate Rust trait bounds syntax
   - [ ] Convert `T: Display` to Rust `T: Display`
   - [ ] Handle multiple bounds `T: Display + Clone`
   - [ ] Generate bounds in function signatures, structs, impls

4. **Testing**:
   - [ ] Unit tests for parser
   - [ ] Integration tests for codegen
   - [ ] Example programs demonstrating trait bounds

**Files to Modify**:
- `src/parser.rs`: Parse trait bound syntax
- `src/codegen.rs`: Generate Rust trait bounds
- `tests/feature_tests.rs`: Add trait bound tests
- `examples/24_trait_bounds/main.wj`: Example demonstrating bounds

### Level 2: Where Clauses

**Goal**: Support complex trait bounds with `where` clauses for readability.

**Syntax**:
```windjammer
fn complex_operation<T, U>(t: T, u: U) -> string
where
    T: Display + Clone,
    U: Debug + PartialEq
{
    format!("${t} and ${u:?}")
}

// Struct with where clause
struct Pair<T, U>
where
    T: Clone,
    U: Debug
{
    first: T,
    second: U
}
```

**Implementation Tasks**:

1. **Parser** (`src/parser.rs`):
   - [x] `FunctionDecl` already has `where_clause: Vec<(String, Vec<String>)>`
   - [ ] Add `where_clause` to `StructDecl` and `ImplBlock`
   - [ ] Parse `where` keyword after signature
   - [ ] Parse comma-separated type constraints
   - [ ] Parse multi-line where clauses

2. **Codegen** (`src/codegen.rs`):
   - [ ] Generate `where` clauses for functions
   - [ ] Generate `where` clauses for structs
   - [ ] Generate `where` clauses for impl blocks
   - [ ] Format multi-line where clauses

3. **Testing**:
   - [ ] Parser tests for where clause syntax
   - [ ] Codegen tests for where clause output
   - [ ] Examples with complex where clauses

**Files to Modify**:
- `src/parser.rs`: Parse where clauses
- `src/codegen.rs`: Generate where clauses
- `tests/feature_tests.rs`: Add where clause tests
- `examples/25_where_clauses/main.wj`: Example

## 🏗️ Phase 2: Associated Types

**Goal**: Support associated types in traits for flexible, type-safe abstractions.

**Syntax**:
```windjammer
trait Container {
    type Item;
    
    fn get(&self) -> &Self::Item;
    fn set(&mut self, item: Self::Item);
}

impl Container for Box<int> {
    type Item = int;
    
    fn get(&self) -> &int {
        &**self
    }
    
    fn set(&mut self, item: int) {
        **self = item
    }
}

// Using associated types in function signatures
fn process<C: Container>(container: &C) -> &C::Item {
    container.get()
}
```

**Implementation Tasks**:

1. **AST Extensions** (`src/parser.rs`):
   - [ ] Add `AssociatedType` struct
   - [ ] Extend `TraitDecl` to include associated types
   - [ ] Extend `ImplBlock` to include associated type definitions
   - [ ] Add `Type::Associated(String, String)` for `C::Item` syntax

2. **Parser** (`src/parser.rs`):
   - [ ] Parse `type Name;` in trait definitions
   - [ ] Parse `type Name = ConcreteType;` in impl blocks
   - [ ] Parse `Self::AssocType` and `T::AssocType` in signatures
   - [ ] Handle associated types in return types and parameters

3. **Analyzer** (`src/analyzer.rs`):
   - [ ] Validate associated type declarations
   - [ ] Check that impl blocks provide all associated types
   - [ ] Validate associated type usage

4. **Codegen** (`src/codegen.rs`):
   - [ ] Generate `type Item;` in trait definitions
   - [ ] Generate `type Item = ConcreteType;` in impl blocks
   - [ ] Convert `Self::Item` to Rust syntax
   - [ ] Handle associated types in bounds

5. **Testing**:
   - [ ] Parser tests for associated types
   - [ ] Full integration tests
   - [ ] Example with Iterator-like trait

**Files to Modify**:
- `src/parser.rs`: Parse associated type syntax
- `src/analyzer.rs`: Validate associated types
- `src/codegen.rs`: Generate associated types
- `tests/feature_tests.rs`: Add associated type tests
- `examples/26_associated_types/main.wj`: Example

**Complexity**: High - requires significant AST changes and validation logic.

## 🏗️ Phase 3: Error Mapping Phase 2

**Goal**: Enhanced error messages with pattern detection and suggestions.

**Features**:

1. **Common Error Patterns**:
   - Detect `&String` vs `&str` confusion → suggest using `&string`
   - Detect missing trait bounds → suggest adding bounds
   - Detect lifetime issues → suggest simplifying ownership
   - Detect closure type inference failures → suggest explicit types

2. **Contextual Suggestions**:
   ```
   error: Type mismatch: expected &str, found &String
     --> main.wj:10:5
   10 |     let x: &str = &name
      |                   ^^^^^ 
   help: In Windjammer, use `&string` which maps to `&str` in Rust
   ```

3. **Rust Concept Translation**:
   - Translate Rust-specific errors to Windjammer concepts
   - Provide links to Windjammer documentation
   - Suggest idiomatic Windjammer patterns

**Implementation Tasks**:

1. **Error Pattern Database** (`src/error_mapper.rs`):
   - [ ] Create `ErrorPattern` struct with matchers and suggestions
   - [ ] Build pattern database for common errors
   - [ ] Implement pattern matching against diagnostics

2. **Enhanced Translation** (`src/main.rs`):
   - [ ] Detect error patterns in diagnostic messages
   - [ ] Generate contextual suggestions
   - [ ] Format suggestions with colors and examples

3. **Documentation Links**:
   - [ ] Map error patterns to GUIDE.md sections
   - [ ] Generate helpful links in error output

**Files to Modify**:
- `src/error_mapper.rs`: Pattern database and matching
- `src/main.rs`: Enhanced error translation
- `docs/ERROR_MAPPING.md`: Update with Phase 2 details

## 🏗️ Phase 4: Stdlib Expansion

**Goal**: Add more standard library modules to increase "batteries included" coverage.

**Priority Modules**:

1. **`std/collections`**:
   - HashMap, HashSet, BTreeMap, BTreeSet wrappers
   - Idiomatic Windjammer APIs

2. **`std/async`** (experimental):
   - Basic async/await support
   - Future trait wrappers

3. **`std/testing`**:
   - Testing framework integration
   - Assertion helpers
   - Mock/spy utilities

4. **Enhanced existing modules**:
   - `std/fs`: Add more file operations
   - `std/http`: Add async support
   - `std/json`: Improve error handling

**Implementation**:
- [ ] Create new stdlib modules
- [ ] Write comprehensive tests
- [ ] Add examples for each module
- [ ] Update stdlib README

## 📊 Success Criteria

### Must Have (v0.8.0 Release Blockers)

- ✅ Trait bounds (Level 1) fully working
- ✅ Where clauses fully working  
- ✅ Associated types fully working
- ✅ All tests passing (100%)
- ✅ Zero clippy warnings
- ✅ Documentation updated
- ✅ At least 3 examples demonstrating new features

### Should Have (Nice to Have)

- ✅ Error mapping Phase 2 implemented
- ✅ 2-3 new stdlib modules
- ✅ Performance benchmarks updated
- ✅ Comprehensive GUIDE.md updates

### Could Have (Future)

- Enhanced trait bound inference
- Named bound sets (ergonomic aliases)
- More stdlib modules

## 🧪 Testing Strategy

### Unit Tests

**Parser Tests** (`tests/parser_tests.rs` - new file):
- [ ] Parse trait bounds on type parameters
- [ ] Parse multiple bounds with `+`
- [ ] Parse where clauses
- [ ] Parse associated types in traits
- [ ] Parse associated type implementations

**Analyzer Tests** (`tests/analyzer_tests.rs`):
- [ ] Validate trait bound references
- [ ] Check associated type completeness
- [ ] Detect missing bounds

### Integration Tests

**Feature Tests** (`tests/feature_tests.rs`):
- [ ] `test_trait_bounds_simple`
- [ ] `test_trait_bounds_multiple`
- [ ] `test_where_clause_function`
- [ ] `test_where_clause_struct`
- [ ] `test_associated_type_trait`
- [ ] `test_associated_type_impl`

### Example Programs

- [ ] `examples/24_trait_bounds/main.wj` - Basic bounds
- [ ] `examples/25_where_clauses/main.wj` - Complex where clauses
- [ ] `examples/26_associated_types/main.wj` - Iterator-like trait
- [ ] `examples/27_advanced_traits/main.wj` - Combining all features

## 📝 Documentation Updates

### Files to Update

- [ ] `CHANGELOG.md` - Add v0.8.0 entry
- [ ] `docs/GUIDE.md` - Add trait bounds, where clauses, associated types sections
- [ ] `README.md` - Update features list
- [ ] `docs/TRAIT_BOUNDS_DESIGN.md` - Mark Level 1 & 2 as implemented
- [ ] Create `docs/ASSOCIATED_TYPES.md` - Design and examples

### New Documentation

- [ ] Tutorial: "Advanced Trait Usage in Windjammer"
- [ ] Guide section: "Trait Bounds Best Practices"
- [ ] Examples with detailed comments

## 🔄 Development Workflow

### Phase Order

1. **Week 1**: Trait Bounds Level 1 & 2
   - Days 1-2: Parser implementation
   - Days 3-4: Codegen implementation
   - Days 5-7: Testing and examples

2. **Week 2**: Associated Types
   - Days 1-3: AST extensions and parser
   - Days 4-5: Codegen and analyzer
   - Days 6-7: Testing and examples

3. **Week 3**: Error Mapping & Stdlib
   - Days 1-3: Error pattern system
   - Days 4-5: Stdlib modules
   - Days 6-7: Documentation, polish, release prep

### Git Workflow

- One commit per logical feature
- Comprehensive commit messages
- Test all changes before committing
- Keep CI green at all times

### Review Checkpoints

- [ ] After Phase 1: Review parser and codegen
- [ ] After Phase 2: Review full type system integration
- [ ] Before merge: Full test suite, documentation review

## 🚀 Release Checklist

Before merging to main:

- [ ] All tests passing (100%)
- [ ] Zero clippy warnings
- [ ] All documentation updated
- [ ] CHANGELOG.md complete
- [ ] Examples working and tested
- [ ] Performance benchmarks run
- [ ] Release notes written
- [ ] PR comment prepared

## 🎯 Stretch Goals

If time permits:

1. **Trait Bound Inference** (Level 3 from design doc):
   - Infer bounds from usage
   - Reduce explicit annotations

2. **Named Bound Sets**:
   - `bound Printable = Display + Debug`
   - `fn log<T: Printable>(value: T)`

3. **Lifetime Annotations** (experimental):
   - Basic lifetime syntax
   - Inference where possible

4. **Generic Enums**:
   - `enum Result<T, E> { Ok(T), Err(E) }`
   - Already have basic enum support, add generics

## 📚 References

- `docs/TRAIT_BOUNDS_DESIGN.md` - 80/20 design philosophy
- Rust Book: Chapter 10 (Generic Types, Traits, and Lifetimes)
- Rust RFC 195: Associated Items
- Previous PRs: v0.6.0 (basic generics), v0.7.0 (turbofish)

---

**Status**: Planning Complete ✅  
**Next Step**: Begin Phase 1 - Trait Bounds Level 1  
**Branch**: `feature/v0.8.0-trait-system`  
**Target Release**: 2-3 weeks from start

