# AST Refactoring Analysis & Plan

**Date:** December 15, 2025  
**Status:** Analysis Complete, Ready for Implementation  
**Estimated Effort:** 8-12 hours (Major Epic)

---

## Executive Summary

The Windjammer AST (`src/parser/ast.rs`, 672 lines) is a **monolithic file** containing 27 types (enums and structs) that represent the entire language syntax tree. While not as large as `generator.rs` was initially, the AST has significant complexity that impacts:

1. **Test Ergonomics** - Constructing AST nodes in tests is verbose and error-prone
2. **Maintainability** - All AST types in one file makes navigation difficult
3. **Cognitive Load** - Understanding relationships between types requires scrolling
4. **Extensibility** - Adding new language features requires modifying a large file

**Recommendation:** Refactor AST into focused, domain-specific modules with builder patterns and comprehensive TDD.

---

## Current State Analysis

### File Structure

```
src/parser/
├── ast.rs                  (672 lines) ← MONOLITHIC
├── expression_parser.rs    (1825 lines)
├── item_parser.rs          (996 lines)
├── statement_parser.rs     (579 lines)
├── pattern_parser.rs       (459 lines)
├── type_parser.rs          (467 lines)
└── mod.rs                  (38 lines)

Total: 5,036 lines
```

### AST Types (27 Total)

#### Core Enums (5)
1. **`Type`** - 17 variants (Int, Float, Bool, Custom, Generic, Parameterized, etc.)
2. **`Statement`** - 17 variants (Let, Const, Return, If, While, For, Match, etc.)
3. **`Expression`** - 21 variants (Literal, Binary, Call, MethodCall, FieldAccess, etc.)
4. **`Pattern`** - 7 variants (Wildcard, Identifier, Tuple, Struct, Enum, etc.)
5. **`Literal`** - 6 variants (Int, Float, String, Bool, Char, Array)

#### Supporting Structs (15)
- `TypeParam`, `AssociatedType`, `OwnershipHint`
- `Parameter`, `Decorator`
- `FunctionDecl`, `StructField`, `StructDecl`
- `EnumVariant`, `EnumVariantData`, `EnumDecl`
- `MatchArm`, `EnumPatternBinding`
- `CompoundOp`, `MacroDelimiter`

#### Auxiliary Types (7)
- `Item`, `TraitDecl`, `ImplDecl`, `UseDeclaration`
- `BinaryOp`, `UnaryOp`, `Program`

---

## Problems Identified

### 1. **Test Construction Complexity**

**Current Reality:**
```rust
// Creating a simple "x + 1" expression requires ~15 lines
let expr = Expression::Binary {
    left: Box::new(Expression::Identifier {
        name: "x".to_string(),
        location: None,
    }),
    op: BinaryOp::Add,
    right: Box::new(Expression::Literal {
        value: Literal::Int(1),
        location: None,
    }),
    location: None,
};
```

**Issues:**
- Verbose and repetitive
- Easy to forget required fields (`location`, `type_args`, etc.)
- Hard to read and understand intent
- Discourages comprehensive test coverage

### 2. **Monolithic File Structure**

**Current:**
- All 27 types in one 672-line file
- No logical grouping
- Hard to navigate
- Difficult to understand relationships

**Impact:**
- Cognitive overload when working with AST
- Harder to onboard new contributors
- Increases likelihood of errors

### 3. **No Builder Patterns**

**Missing:**
- No fluent API for constructing AST nodes
- No sensible defaults (e.g., `location: None` everywhere)
- No type-safe construction helpers

**Result:**
- Tests are brittle and hard to maintain
- Refactoring AST structure breaks many tests
- Discourages TDD

### 4. **Lack of Domain Separation**

**Current:** All types mixed together
**Better:** Group by domain:
- Type system (`Type`, `TypeParam`, `AssociatedType`)
- Expressions (`Expression`, `Literal`, `BinaryOp`, `UnaryOp`)
- Statements (`Statement`, `CompoundOp`)
- Patterns (`Pattern`, `EnumPatternBinding`)
- Declarations (`FunctionDecl`, `StructDecl`, `EnumDecl`, etc.)
- Ownership (`OwnershipHint`, `Parameter`)

---

## Proposed Solution: Modular AST with Builders

### Phase 1: Domain Separation (Structural Refactor)

Break `ast.rs` into focused modules:

```
src/parser/ast/
├── mod.rs                  (Re-exports all types)
├── types.rs                (Type, TypeParam, AssociatedType)
├── expressions.rs          (Expression, Literal, BinaryOp, UnaryOp)
├── statements.rs           (Statement, CompoundOp)
├── patterns.rs             (Pattern, EnumPatternBinding, MatchArm)
├── declarations.rs         (FunctionDecl, StructDecl, EnumDecl, etc.)
├── ownership.rs            (OwnershipHint, Parameter)
├── program.rs              (Program, Item, UseDeclaration)
└── builders/               (Builder patterns for each domain)
    ├── mod.rs
    ├── expression_builder.rs
    ├── statement_builder.rs
    ├── pattern_builder.rs
    └── declaration_builder.rs
```

**Estimated Effort:** 2-3 hours
**Tests:** Move existing tests, ensure zero regressions

### Phase 2: Builder Patterns (Ergonomics Improvement)

Create fluent builder APIs for common AST construction:

```rust
// BEFORE (15 lines)
let expr = Expression::Binary {
    left: Box::new(Expression::Identifier {
        name: "x".to_string(),
        location: None,
    }),
    op: BinaryOp::Add,
    right: Box::new(Expression::Literal {
        value: Literal::Int(1),
        location: None,
    }),
    location: None,
};

// AFTER (1 line)
let expr = ExprBuilder::binary(
    ExprBuilder::ident("x"),
    BinaryOp::Add,
    ExprBuilder::int(1)
);
```

**Builder API Design:**

```rust
pub struct ExprBuilder;

impl ExprBuilder {
    // Literals
    pub fn int(n: i64) -> Expression { /* ... */ }
    pub fn float(f: f64) -> Expression { /* ... */ }
    pub fn string(s: impl Into<String>) -> Expression { /* ... */ }
    pub fn bool(b: bool) -> Expression { /* ... */ }
    
    // Identifiers
    pub fn ident(name: impl Into<String>) -> Expression { /* ... */ }
    
    // Binary operations
    pub fn binary(left: Expression, op: BinaryOp, right: Expression) -> Expression { /* ... */ }
    pub fn add(left: Expression, right: Expression) -> Expression { /* ... */ }
    pub fn sub(left: Expression, right: Expression) -> Expression { /* ... */ }
    
    // Function calls
    pub fn call(func: Expression, args: Vec<Expression>) -> Expression { /* ... */ }
    pub fn method_call(obj: Expression, method: impl Into<String>, args: Vec<Expression>) -> Expression { /* ... */ }
    
    // Field access
    pub fn field(obj: Expression, field: impl Into<String>) -> Expression { /* ... */ }
    
    // Blocks
    pub fn block(stmts: Vec<Statement>) -> Expression { /* ... */ }
}

pub struct StmtBuilder;

impl StmtBuilder {
    pub fn let_(name: impl Into<String>, value: Expression) -> Statement { /* ... */ }
    pub fn return_(expr: Expression) -> Statement { /* ... */ }
    pub fn if_(cond: Expression, then: Vec<Statement>, else_: Option<Vec<Statement>>) -> Statement { /* ... */ }
    pub fn assignment(target: Expression, value: Expression) -> Statement { /* ... */ }
    pub fn expr(expr: Expression) -> Statement { /* ... */ }
}

pub struct PatternBuilder;

impl PatternBuilder {
    pub fn wildcard() -> Pattern { /* ... */ }
    pub fn ident(name: impl Into<String>) -> Pattern { /* ... */ }
    pub fn tuple(patterns: Vec<Pattern>) -> Pattern { /* ... */ }
    pub fn struct_(name: impl Into<String>, fields: Vec<(String, Pattern)>) -> Pattern { /* ... */ }
}

pub struct DeclBuilder;

impl DeclBuilder {
    pub fn function(name: impl Into<String>) -> FunctionDeclBuilder { /* ... */ }
    pub fn struct_(name: impl Into<String>) -> StructDeclBuilder { /* ... */ }
}

// Fluent builder for complex declarations
pub struct FunctionDeclBuilder {
    name: String,
    params: Vec<Parameter>,
    return_type: Option<Type>,
    body: Vec<Statement>,
    is_pub: bool,
    // ... other fields
}

impl FunctionDeclBuilder {
    pub fn param(mut self, name: impl Into<String>, type_: Type) -> Self { /* ... */ }
    pub fn returns(mut self, type_: Type) -> Self { /* ... */ }
    pub fn body(mut self, stmts: Vec<Statement>) -> Self { /* ... */ }
    pub fn public(mut self) -> Self { /* ... */ }
    pub fn build(self) -> FunctionDecl { /* ... */ }
}
```

**Estimated Effort:** 4-5 hours
**Tests:** TDD for each builder (100+ tests)

### Phase 3: Refactor Existing Tests (Test Modernization)

Update all existing tests to use builders:

**Before:**
```rust
#[test]
fn test_binary_op_ownership() {
    let expr = Expression::Binary {
        left: Box::new(Expression::Identifier {
            name: "a".to_string(),
            location: None,
        }),
        op: BinaryOp::Add,
        right: Box::new(Expression::Identifier {
            name: "b".to_string(),
            location: None,
        }),
        location: None,
    };
    // ... test logic
}
```

**After:**
```rust
#[test]
fn test_binary_op_ownership() {
    let expr = ExprBuilder::add(
        ExprBuilder::ident("a"),
        ExprBuilder::ident("b")
    );
    // ... test logic
}
```

**Estimated Effort:** 2-3 hours
**Tests:** All existing tests should still pass

### Phase 4: Documentation & Examples (Knowledge Transfer)

Create comprehensive documentation:

1. **AST Architecture Guide** - Explain module structure
2. **Builder API Reference** - Document all builder methods
3. **Testing Best Practices** - Show how to use builders in tests
4. **Migration Guide** - Help contributors update old code

**Estimated Effort:** 1-2 hours

---

## Implementation Plan

### Prerequisites
- ✅ All compiler tests passing (248 tests)
- ✅ All module tests passing (196 tests)
- ✅ Zero regressions from previous refactoring

### Execution Strategy

#### Phase 1: Structural Refactor (TDD)
1. Create `src/parser/ast/` directory
2. Create `mod.rs` with re-exports
3. Move `Type` + related types to `types.rs`
4. Update imports, run tests (should pass)
5. Move `Expression` + related to `expressions.rs`
6. Update imports, run tests (should pass)
7. Repeat for all domains
8. Delete old `ast.rs`
9. Final test run (all 444 tests should pass)

**Commit:** "refactor(ast): Break monolithic ast.rs into domain modules"

#### Phase 2: Builder Implementation (TDD)
1. Create `builders/` directory
2. Write tests for `ExprBuilder` (20+ tests)
3. Implement `ExprBuilder` to pass tests
4. Write tests for `StmtBuilder` (15+ tests)
5. Implement `StmtBuilder` to pass tests
6. Write tests for `PatternBuilder` (10+ tests)
7. Implement `PatternBuilder` to pass tests
8. Write tests for `DeclBuilder` (15+ tests)
9. Implement `DeclBuilder` to pass tests
10. Final test run (all tests + 60+ new tests should pass)

**Commit:** "feat(ast): Add builder patterns for ergonomic AST construction"

#### Phase 3: Test Modernization
1. Identify all tests constructing AST manually
2. Refactor 10-20 tests at a time
3. Run tests after each batch
4. Ensure zero regressions

**Commit:** "test: Modernize tests to use AST builders"

#### Phase 4: Documentation
1. Write AST architecture guide
2. Document builder API
3. Create testing best practices doc
4. Write migration guide

**Commit:** "docs: Comprehensive AST refactoring documentation"

---

## Success Criteria

### Quantitative
- ✅ All 444 existing tests passing
- ✅ 60+ new builder tests passing
- ✅ Zero regressions
- ✅ AST construction code reduced by 60-80%

### Qualitative
- ✅ Tests are easier to read and write
- ✅ AST structure is more maintainable
- ✅ New contributors can understand AST quickly
- ✅ Adding new language features is easier

---

## Risks & Mitigation

### Risk 1: Breaking Changes
**Mitigation:** Use re-exports in `mod.rs` to maintain backward compatibility

### Risk 2: Test Failures
**Mitigation:** Refactor incrementally, run tests after each step

### Risk 3: Incomplete Builder Coverage
**Mitigation:** Start with most common patterns, expand iteratively

### Risk 4: Time Overrun
**Mitigation:** Phase 1 is MVP, Phases 2-4 can be done incrementally

---

## Comparison: Before vs After

### Before (Current State)
```rust
// Test code: 25 lines for simple function
#[test]
fn test_function_with_params() {
    let func = FunctionDecl {
        name: "add".to_string(),
        is_pub: true,
        is_extern: false,
        parameters: vec![
            Parameter {
                name: "a".to_string(),
                pattern: None,
                type_: Type::Int,
                ownership: OwnershipHint::Inferred,
                is_mutable: false,
            },
            Parameter {
                name: "b".to_string(),
                pattern: None,
                type_: Type::Int,
                ownership: OwnershipHint::Inferred,
                is_mutable: false,
            },
        ],
        return_type: Some(Type::Int),
        body: vec![
            Statement::Return {
                value: Some(Expression::Binary {
                    left: Box::new(Expression::Identifier {
                        name: "a".to_string(),
                        location: None,
                    }),
                    op: BinaryOp::Add,
                    right: Box::new(Expression::Identifier {
                        name: "b".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                location: None,
            },
        ],
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        doc_comment: None,
    };
    // ... test logic
}
```

### After (With Builders)
```rust
// Test code: 8 lines for same function
#[test]
fn test_function_with_params() {
    let func = DeclBuilder::function("add")
        .public()
        .param("a", Type::Int)
        .param("b", Type::Int)
        .returns(Type::Int)
        .body(vec![
            StmtBuilder::return_(ExprBuilder::add(
                ExprBuilder::ident("a"),
                ExprBuilder::ident("b")
            ))
        ])
        .build();
    // ... test logic
}
```

**Reduction:** 25 lines → 8 lines (68% reduction)
**Clarity:** Intent is immediately clear
**Maintainability:** Changes to AST structure require minimal test updates

---

## Timeline Estimate

| Phase | Description | Estimated Time | Cumulative |
|-------|-------------|----------------|------------|
| 1 | Structural Refactor | 2-3 hours | 2-3 hours |
| 2 | Builder Implementation | 4-5 hours | 6-8 hours |
| 3 | Test Modernization | 2-3 hours | 8-11 hours |
| 4 | Documentation | 1-2 hours | 9-13 hours |

**Total Estimate:** 9-13 hours (Major Epic)

---

## Recommendation

**PROCEED with AST Refactoring**

**Rationale:**
1. **High Impact** - Improves test ergonomics across entire codebase
2. **Aligns with Philosophy** - Better maintainability, clearer code
3. **Enables Future Work** - Makes language evolution easier
4. **TDD-Friendly** - Builders make comprehensive testing easier

**Priority:** HIGH (after current refactoring session)

**Next Steps:**
1. Complete current generator.rs refactoring
2. Commit all pending work
3. Start Phase 1: Structural Refactor with TDD
4. Proceed through phases systematically

---

## Appendix: Related Work

### Similar Patterns in Other Projects

**Rust Compiler (`rustc`):**
- Uses builder patterns extensively (`ty::TyCtxt`, `hir::Builder`)
- Separates HIR (High-level IR) from AST
- Comprehensive testing infrastructure

**TypeScript Compiler:**
- Factory functions for AST construction (`ts.factory.createXxx()`)
- Reduces boilerplate in tests and transformations
- Well-documented builder API

**Babel (JavaScript):**
- `@babel/types` package with builder functions
- `t.identifier()`, `t.binaryExpression()`, etc.
- Industry standard for AST manipulation

**Windjammer Approach:**
- Combine best practices from all three
- Rust-idiomatic builder patterns
- TDD-driven implementation
- Comprehensive documentation

---

**Status:** Ready for Implementation  
**Author:** AI Assistant (with user guidance)  
**Date:** December 15, 2025










