# Windjammer v0.10.0 Development Plan

## 🎯 Theme: Inference Over Annotation

**Goal**: Implement automatic trait bound inference to achieve "80% simplicity through 80% inference"

**Timeline**: 2-3 weeks  
**Branch**: `feature/v0.10.0-inference`

---

## 📋 Features Overview

| Feature | Priority | Complexity | Status |
|---------|----------|------------|--------|
| Inferred Trait Bounds (FLAGSHIP) | High | High | 🔜 Planned |
| Named Bound Sets | High | Low | 🔜 Planned |
| Enhanced @test Decorator | Medium | Medium | 🔜 Planned |
| Enhanced @async Decorator | Medium | Medium | 🔜 Planned |
| Error Mapping Improvements | Medium | Low | 🔜 Planned |
| Documentation & Examples | High | Medium | 🔜 Planned |

---

## 🏗️ Phase 1: Inferred Trait Bounds (FLAGSHIP)

### Goal

**80% of developers never write trait bounds explicitly.**

Allow users to write:
```windjammer
fn print_all<T>(items: Vec<T>) {
    for item in items {
        println!("{}", item)  // Compiler infers T: Display
    }
}
```

Instead of:
```windjammer
fn print_all<T: Display>(items: Vec<T>) {
    for item in items {
        println!("{}", item)
    }
}
```

### Implementation Steps

#### Week 1: Foundation (Days 1-3)

**Day 1: Infrastructure**
- [ ] Create `src/inference.rs` module
- [ ] Add `TraitConstraint` struct
- [ ] Add `InferredBounds` struct  
- [ ] Integrate inference phase into main pipeline

**Day 2: Basic Trait Detection**
- [ ] Implement constraint collector for function bodies
- [ ] Add detection for `Display` (println!("{}", x))
- [ ] Add detection for `Debug` (println!("{:?}", x))
- [ ] Add detection for `Clone` (x.clone())
- [ ] Unit tests for each detector

**Day 3: Binary Operators**
- [ ] Add detection for `Add` (x + y)
- [ ] Add detection for `Sub` (x - y)
- [ ] Add detection for `Mul` (x * y)
- [ ] Add detection for `Div` (x / y)
- [ ] Add detection for `PartialEq` (x == y, x != y)
- [ ] Add detection for `PartialOrd` (x < y, x > y, etc.)
- [ ] Unit tests for operators

#### Week 2: Integration (Days 4-7)

**Day 4: Iteration & Loops**
- [ ] Add detection for `IntoIterator` (for x in items)
- [ ] Add detection for `Iterator` (.iter(), .iter_mut())
- [ ] Handle edge cases (iterating over references)
- [ ] Unit tests

**Day 5: Codegen Integration**
- [ ] Update `codegen.rs` to accept inferred bounds
- [ ] Merge explicit + inferred bounds
- [ ] Generate sorted trait bounds for stability
- [ ] Integration tests with real examples

**Day 6: Error Handling**
- [ ] Detect inference failures
- [ ] Generate helpful error messages
- [ ] Update error mapper for inference errors
- [ ] Test error cases

**Day 7: Testing & Refinement**
- [ ] Comprehensive test suite
- [ ] Test with all 33 existing examples
- [ ] Performance testing
- [ ] Bug fixes

#### Week 3: Named Bounds & Polish (Days 8-10)

**Day 8: Named Bound Sets**
- [ ] Add `bound` keyword to lexer
- [ ] Parse `bound Name = Trait + Trait` declarations
- [ ] Add `BoundAlias` to AST
- [ ] Expand aliases in codegen
- [ ] Unit tests

**Day 9: Examples & Documentation**
- [ ] Create Example 34: Inferred Trait Bounds
- [ ] Create Example 35: Named Bound Sets
- [ ] Update GUIDE.md with inference chapter
- [ ] Update README.md
- [ ] Migration guide for v0.9.0 users

**Day 10: Final Polish**
- [ ] Run all tests
- [ ] Fix any remaining issues
- [ ] Update CHANGELOG.md
- [ ] Performance benchmarks
- [ ] Prepare PR

---

## 🏗️ Phase 2: Enhanced Decorators

### @test Decorator

**Goal**: Reduce test boilerplate

```windjammer
// Write this:
@test
fn test_addition() {
    assert_eq(add(2, 2), 4)
}

// Get this:
#[test]
fn test_addition() {
    assert_eq!(add(2, 2), 4);
}
```

**Implementation**:
- Parse `@test` decorator on functions
- Generate `#[test]` attribute in Rust
- Handle `@test(name = "custom name")` variants
- Example 36: Testing with @test decorator

### @async Decorator

**Goal**: Simplify async functions

```windjammer
// Write this:
@async
fn fetch_data(url: string) -> string {
    http::get(url).await
}

// Get this:
#[tokio::main]
async fn fetch_data(url: String) -> String {
    http::get(url).await
}
```

**Implementation**:
- Parse `@async` decorator
- Generate `async fn` in Rust
- Handle runtime selection (tokio by default)
- Add tokio to Cargo.toml when used
- Example 37: Async functions

---

## 📚 Documentation Updates

### User-Facing

**Update README.md**:
- Add "Automatic Inference" section prominently
- Show before/after comparisons
- Emphasize "write less, get the same"

**Update GUIDE.md**:
- New Chapter 3: "Generics Without Trait Bounds"
- Move explicit bounds to Chapter 10
- Progressive disclosure approach
- Examples throughout

**Create MIGRATION.md**:
- v0.9.0 → v0.10.0 guide
- When to use explicit bounds
- How inference works
- Troubleshooting

### Developer-Facing

**Update CONTRIBUTING.md**:
- How to add new trait detection rules
- Testing strategy for inference
- Debugging inference failures

**Update INFERENCE_DESIGN.md**:
- Mark Phase 1 as "Implemented"
- Document any deviations from plan
- Lessons learned

---

## ✅ Success Criteria

### Must Have (Release Blockers)
- [ ] Inferred trait bounds working for Display, Debug, Clone
- [ ] Inferred bounds for binary operators (Add, Sub, etc.)
- [ ] Inferred bounds for iteration (IntoIterator)
- [ ] Named bound sets (`bound Name = Trait + Trait`)
- [ ] @test and @async decorators functional
- [ ] All existing 33 examples still work
- [ ] 2 new examples (34-35) demonstrating inference
- [ ] Zero test failures
- [ ] Zero clippy warnings
- [ ] Comprehensive documentation

### Should Have (Nice to Have)
- [ ] Performance overhead <5%
- [ ] 3+ examples with decorators (36-37+)
- [ ] Error messages for inference failures
- [ ] Migration guide complete

### Could Have (Future)
- [ ] Cross-function inference (v0.11.0)
- [ ] Lifetime inference (v0.11.0+)
- [ ] Effect inference (v0.11.0)

---

## 🧪 Testing Strategy

### Unit Tests

**Inference Module**:
- Test each trait detector in isolation
- Test constraint collection
- Test constraint simplification
- Test edge cases (no usage, conflicts, etc.)

**Integration Tests**:
- Test with real function bodies
- Test mixing explicit + inferred bounds
- Test error cases

### Example Tests

**Positive Tests**:
- All 33 existing examples must continue to work
- New examples 34-35 must compile
- Generated Rust must be valid

**Regression Tests**:
- Ensure no breaking changes
- Ensure performance doesn't degrade

---

## 📊 Development Phases

### Week 1: Core Inference Engine
- **Days 1-3**: Build trait bound inference infrastructure
- **Focus**: Get basic inference working (Display, Debug, Clone)
- **Milestone**: Can infer bounds for simple functions

### Week 2: Complete Inference
- **Days 4-7**: Add all trait detectors + integration
- **Focus**: Operators, iteration, error handling
- **Milestone**: All planned traits can be inferred

### Week 3: Polish & Ship
- **Days 8-10**: Named bounds, decorators, docs, examples
- **Focus**: Make it production-ready
- **Milestone**: Ready to merge

---

## 🚀 Release Checklist

Before merging to main:

- [ ] All features implemented and tested
- [ ] All 33 existing examples work unchanged
- [ ] New examples 34-37 work
- [ ] Zero test failures (all 48+ tests passing)
- [ ] Zero clippy warnings
- [ ] `cargo fmt --all` clean
- [ ] Documentation updated:
  - [ ] README.md
  - [ ] CHANGELOG.md
  - [ ] GUIDE.md
  - [ ] INFERENCE_DESIGN.md
- [ ] Performance benchmarks run
- [ ] PR comment prepared
- [ ] Release notes written

---

## 🔮 Looking Ahead to v0.11.0

Features planned for v0.11.0:
- **Implicit Return Types**: Infer from function body
- **Smart Error Propagation**: Infer Result return type from `?`
- **Effect Inference**: Infer Send/Sync from concurrency usage
- **More Decorators**: @benchmark, @memoize
- **Basic Lifetime Inference**: Simple cases beyond Rust elision

---

## 📚 References

- `docs/INFERENCE_DESIGN.md` - Full research and algorithm
- Rust Book: Chapter 10 (Generics, Traits, Lifetimes)
- Previous PRs: v0.9.0 (generic trait impls, generic enums)

---

**Status**: Planning Complete ✅  
**Next Step**: Begin Day 1 - Infrastructure  
**Branch**: `feature/v0.10.0-inference`  
**Target Release**: 2-3 weeks from start

**Core Philosophy**: Progressive disclosure through intelligence, not feature limitation.

