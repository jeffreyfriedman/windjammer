# Technical Debt Tracker

This document tracks known technical debt in the Windjammer compiler that needs to be addressed.

## Active Tech Debt

### 1. UI Integration Tests (Priority: Medium)
**Status**: Quick-fixed with `is_extern: false`  
**TODO ID**: `fix-ui-integration-tests`, `remove-component-decorator`

**Problem**:
- `tests/ui_integration_tests.rs` contains outdated tests for `@component` decorator
- Quick-fixed by adding `is_extern: false` to all `FunctionDecl` structs
- Tests still fail compilation (26 errors remaining)

**Root Cause**:
- `@component` decorator is being removed from the compiler (v0.40+)
- Tests were written for old UI system that's being replaced
- `FunctionDecl` struct evolved but tests weren't updated

**Proper Fix**:
1. Remove `@component` decorator support from compiler
2. Remove `tests/ui_integration_tests.rs` entirely
3. Remove `src/ui/` directory
4. Remove `Signal` type mapping from `types.rs`
5. Remove `windjammer-ui` from WASM Cargo.toml generation
6. Implement new template-based decorator system (v0.40+)

**Impact**:
- Low: Tests are for deprecated features
- Library compiles fine, only test compilation fails
- Pattern matching tests (20/20) pass successfully

**Timeline**: Address in v0.40 cleanup phase

---

## Resolved Tech Debt

### Pattern Matching Implementation ✅
**Resolved**: November 29, 2025

- Implemented tuple enum variants
- Added refutable pattern detection
- Created comprehensive test suite (20/20 passing)
- No workarounds, proper implementation

---

## Guidelines

**When Adding Tech Debt**:
1. Document the problem clearly
2. Explain why the quick fix was chosen
3. Describe the proper long-term solution
4. Assign a priority and timeline
5. Link to related TODO items

**Priorities**:
- **Critical**: Blocks development or causes bugs
- **High**: Affects code quality or maintainability significantly
- **Medium**: Should be fixed but not urgent
- **Low**: Nice to have, can wait

**Our Commitment**:
> "No workarounds, no tech debt, only proper fixes" - unless explicitly documented here with a plan to address it.

---

Last Updated: November 29, 2025














