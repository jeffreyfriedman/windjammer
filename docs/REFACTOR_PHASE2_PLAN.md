# Generator.rs Refactoring - Phase 2

**Current Status:** 5,858 lines (93 functions)  
**Goal:** Extract 6-8 modules, reduce to ~2,000 lines of core logic

---

## Module Extraction Plan

### ✅ Phase 1 Complete
1. **literals.rs** (100 lines, 6 tests) ✅
2. **type_casting.rs** (extracted, needs cleanup) ✅
3. **Framework code removed** (-524 lines) ✅

### 🎯 Phase 2: Self/Field Analysis (Priority: HIGH)
**Target:** ~600 lines → new module  
**Functions to extract (12):**
- `function_accesses_fields()`
- `function_mutates_fields()`
- `function_modifies_self()`
- `statement_modifies_self()`
- `expression_modifies_self()`
- `expression_accesses_fields()`
- `expression_mutates_fields()`
- `statement_accesses_fields()`
- `statement_mutates_fields()`
- `expression_is_self_field_modification()`
- `statement_modifies_variable()`
- `loop_body_modifies_variable()`

**New file:** `src/codegen/rust/self_analysis.rs`

**Why this first?**
- Well-defined concern (self parameter inference)
- High cohesion (all functions analyze AST for mutations)
- Used extensively by ownership inference
- Easy to test (pure functions, AST in → bool out)

### 📦 Phase 3: Type System Module
**Target:** ~500 lines → new module  
**Functions to extract (16):**
- `type_to_rust()`
- `is_copy_type()`, `is_eq_type()`, `is_hashable_type()`, `is_partial_eq_type()`
- `all_fields_are_copy()`, `all_fields_are_eq()`, `all_fields_are_hashable()`, `all_fields_are_partial_eq()`
- `all_enum_variants_are_partial_eq()`
- `infer_derivable_traits()`
- `format_type_params()`, `format_where_clause()`
- `has_default()`, `all_fields_have_default()`

**New file:** `src/codegen/rust/type_system.rs`

### 🎨 Phase 4: Expression Generation
**Target:** ~800 lines → new module  
**Functions to extract (8):**
- `generate_expression()`
- `generate_expression_immut()`
- `generate_expression_with_precedence()`
- `binary_op_to_rust()`, `unary_op_to_rust()`, `op_precedence()`
- `generate_string_concat()`
- `generate_tauri_invoke()`

**New file:** `src/codegen/rust/expressions.rs`

### 🔍 Phase 5: Pattern Matching Module
**Target:** ~300 lines → new module  
**Functions to extract (5):**
- `generate_pattern()`
- `pattern_to_rust()`
- `pattern_extracts_value()`
- `pattern_has_string_literal()`
- `extract_pattern_identifier()`

**New file:** `src/codegen/rust/patterns.rs`

### 🔤 Phase 6: String Conversion Module
**Target:** ~400 lines → new module  
**Functions to extract (8):**
- `expression_produces_string()`
- `contains_string_literal()`
- `block_has_as_str()`, `statement_has_as_str()`, `expression_has_as_str()`
- `arm_returns_converted_string()`
- `match_needs_clone_for_self_field()`
- `collect_concat_parts_static()`

**New file:** `src/codegen/rust/string_analysis.rs`

### 🏗️ Phase 7: Statement Generation
**Target:** ~500 lines → new module  
**Functions to extract (3):**
- `generate_statement()`
- `generate_statement_tracked()`
- `generate_block()`

**New file:** `src/codegen/rust/statements.rs`

### 📐 Phase 8: Type Declarations
**Target:** ~600 lines → new module  
**Functions to extract (5):**
- `generate_struct()`
- `generate_enum()`
- `generate_trait()`
- `generate_impl()`
- `generate_function()`

**New file:** `src/codegen/rust/declarations.rs`

---

## Testing Strategy

### For Each Module Extraction:

1. **Write tests first (TDD)**:
   - Create `tests/codegen_<module>_test.rs`
   - Test core functionality with realistic inputs
   - Aim for 80%+ coverage

2. **Extract module**:
   - Move functions to new file
   - Update `mod.rs` with `pub mod <module>;`
   - Export functions with `pub(super)`

3. **Update generator.rs**:
   - Replace function bodies with `use super::<module>::*;`
   - Ensure all call sites work

4. **Verify**:
   - Run `cargo test --lib`
   - Run `cargo clippy`
   - Check line count reduction

5. **Commit**:
   - `refactor: Extract <module> from generator.rs (-XXX lines)`

---

## Expected Results

| Phase | Lines Extracted | Remaining | Tests Added |
|-------|----------------|-----------|-------------|
| Phase 1 (Done) | -524 | 5,858 | 6 |
| Phase 2 (Self) | -600 | ~5,258 | 12 |
| Phase 3 (Types) | -500 | ~4,758 | 16 |
| Phase 4 (Expr) | -800 | ~3,958 | 8 |
| Phase 5 (Patterns) | -300 | ~3,658 | 5 |
| Phase 6 (Strings) | -400 | ~3,258 | 8 |
| Phase 7 (Stmt) | -500 | ~2,758 | 3 |
| Phase 8 (Decl) | -600 | ~2,158 | 5 |
| **TOTAL** | **-3,724** | **~2,158** | **63** |

**Final state:**
- `generator.rs`: ~2,158 lines (core orchestration)
- 8 focused modules: ~3,700 lines (extracted logic)
- 63+ new tests (comprehensive coverage)

---

## Benefits

✅ **Maintainability** - Smaller files, easier to navigate  
✅ **Testability** - Each module tested independently  
✅ **Clarity** - Clear separation of concerns  
✅ **Reusability** - Modules can be used by other codegen passes  
✅ **Velocity** - Easier to add features without conflicts  

---

## Next Steps

1. Start with Phase 2 (Self Analysis) - highest impact
2. Use strict TDD for each extraction
3. Run full test suite after each phase
4. Document each module with clear docs

**Let's begin!** 🚀










