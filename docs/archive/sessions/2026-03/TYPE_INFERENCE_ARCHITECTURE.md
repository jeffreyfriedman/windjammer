# Windjammer Type Inference Architecture

## Problem Statement

**Current (Function-Level Inference):**
```windjammer
fn foo() -> f32 { 0.0 }  // All literals in foo → f32
fn bar() -> f64 { 1.0 }  // All literals in bar → f64
let x = foo() * bar()    // ERROR: f32 * f64
```

**Desired (Expression-Level Inference):**
```windjammer
fn foo() -> f32 { 0.0_f32 }  // ✅ Inferred from return
fn bar() -> f64 { 1.0_f64 }  // ✅ Inferred from return
// Windjammer ERROR: "Cannot multiply f32 by f64. Consider: foo() as f64"
```

---

## Solution: Constraint-Based Type Inference

### Architecture

```
Parser → Type Inference Pass → Analyzer → Codegen → Backend
              ↓
         Constraint Collection
              ↓
         Constraint Solving (Unification)
              ↓
         Error Detection (Mixing)
              ↓
         Type Annotations (ExprId → FloatType)
```

### Data Structures

```rust
// Unique identifier for expressions
struct ExprId { line: usize, col: usize }

// Float type for an expression
enum FloatType { F32, F64, Unknown }

// Constraints collected during traversal
enum Constraint {
    MustBeF32(ExprId, String),  // Expression must be f32 (reason)
    MustBeF64(ExprId, String),  // Expression must be f64 (reason)
    MustMatch(ExprId, ExprId, String),  // Two expressions must match (reason)
}

// Inference engine
struct FloatInference {
    inferred_types: HashMap<ExprId, FloatType>,
    constraints: Vec<Constraint>,
    errors: Vec<String>,
}
```

---

## Constraint Collection Rules

### 1. Binary Operations (High Priority)
```windjammer
let x: f32 = 1.0
let y = x * 2.0  // 2.0 MUST BE f32 (matches x)
```
**Constraint:** `MustMatch(x, 2.0, "binary operation")`

### 2. Method Calls (High Priority)
```windjammer
let x: f32 = 1.0
let y = x.max(2.0)  // 2.0 MUST BE f32 (matches x)
```
**Constraint:** `MustMatch(x, 2.0, "method call argument")`

### 3. Function Calls (Medium Priority)
```windjammer
fn foo(a: f32, b: f32) -> f32 { a + b }
let result = foo(1.0, 2.0)  // 1.0 and 2.0 MUST BE f32
```
**Constraint:** `MustBeF32(1.0, "parameter a: f32")`, `MustBeF32(2.0, "parameter b: f32")`

### 4. Return Statements (Medium Priority)
```windjammer
fn foo() -> f32 {
    return 1.0  // 1.0 MUST BE f32
}
```
**Constraint:** `MustBeF32(1.0, "return type f32")`

### 5. Struct Literals (High Priority)
```windjammer
struct Vec3 { x: f32, y: f32, z: f32 }
let v = Vec3 { x: 1.0, y: 2.0, z: 3.0 }  // All MUST BE f32
```
**Constraint:** `MustBeF32(1.0, "field Vec3::x: f32")`, etc.

### 6. Assignments (Medium Priority)
```windjammer
let x: f32 = 1.0
x = 2.0  // 2.0 MUST BE f32 (matches x's type)
```
**Constraint:** `MustMatch(x, 2.0, "assignment")`

### 7. Tuple Construction (Medium Priority)
```windjammer
fn neighbors() -> Vec<(i32, i32, f32)> {
    vec![(0, 0, 1.414)]  // 1.414 MUST BE f32
}
```
**Constraint:** `MustBeF32(1.414, "tuple element 2: f32")`

---

## Constraint Solving Algorithm

### Phase 1: Forward Propagation
```
For each constraint:
  If MustBeF32(expr):
    Mark expr as f32
  If MustBeF64(expr):
    Mark expr as f64
  If MustMatch(expr1, expr2):
    If expr1 is known: Mark expr2 same type
    If expr2 is known: Mark expr1 same type
```

### Phase 2: Conflict Detection
```
For each expression:
  If marked both f32 AND f64:
    ERROR: "Type conflict at line X: cannot be both f32 and f64"
```

### Phase 3: Default Assignment
```
For each unknown expression:
  Assign f64 (Rust/Python/Go standard default)
```

---

## Integration Points

### 1. Parser (No Changes)
- AST already has source locations
- Types already captured

### 2. New Pass: Type Inference (AFTER Parsing, BEFORE Codegen)
```rust
// In compile() function:
let ast = parse(source)?;
let mut inference = FloatInference::new();
inference.infer_program(&ast);
if !inference.errors.is_empty() {
    return Err(inference.errors);
}
// Pass inference to codegen
let rust_code = codegen(ast, inference)?;
```

### 3. Codegen (Uses Inference Results)
```rust
fn generate_literal(&self, lit: &Literal, expr: &Expression) -> String {
    match lit {
        Literal::Float(f) => {
            let float_type = self.float_inference.get_float_type(expr);
            match float_type {
                FloatType::F32 => format!("{}_f32", f),
                FloatType::F64 => format!("{}_f64", f),
                FloatType::Unknown => format!("{}_f64", f), // Default
            }
        }
        // ... other literals
    }
}
```

---

## Error Messages (World-Class Quality)

### Example 1: Mixing Types
```
error: cannot multiply f32 by f64
  --> src/main.wj:5:15
   |
5  |     let result = x * y
   |                  -   - f64 (from variable y: f64)
   |                  |
   |                  f32 (from variable x: f32)
   |
help: consider casting one operand
   |
5  |     let result = (x as f64) * y
   |                  ^^^^^^^^^^
5  |     let result = x * (y as f32)
   |                      ^^^^^^^^^^
```

### Example 2: Ambiguous Context
```
error: ambiguous float literal type
  --> src/main.wj:10:18
   |
10 |     let scale = 2.0
   |                 ^^^ cannot infer if this should be f32 or f64
   |
help: add explicit type annotation
   |
10 |     let scale: f32 = 2.0
   |              ^^^^^
```

---

## Backend Compatibility

### Rust Backend
- **Needs:** Explicit `_f32`/`_f64` suffixes for method calls
- **Strategy:** Use inference results to generate suffixes
- **Example:** `0.0.max(1.0)` → `0.0_f32.max(1.0_f32)`

### Go Backend
- **Needs:** No suffixes (Go infers naturally)
- **Strategy:** Skip suffix generation, use bare `0.0`
- **Example:** `0.0.Max(1.0)` → `0.0.Max(1.0)`

### JavaScript Backend
- **Needs:** No suffixes (only has `number`)
- **Strategy:** Skip suffix generation, use bare `0.0`
- **Example:** `Math.max(0.0, 1.0)` → `Math.max(0.0, 1.0)`

### Interpreter Backend
- **Needs:** No suffixes (runtime typed)
- **Strategy:** Skip suffix generation
- **Example:** `0.0` → `0.0`

**Key Insight:** Type inference runs **before** backend codegen, so results can be used differently per backend!

---

## Implementation Phases

### Phase 1: Basic Constraint Collection ✅ (Done)
- [x] Binary operations
- [x] Method calls
- [x] Return statements
- [x] Struct literals
- [ ] Function calls (parameters)
- [ ] Assignments
- [ ] Tuple construction

### Phase 2: Constraint Solving ✅ (Done)
- [x] Unification algorithm
- [x] Conflict detection
- [x] Default assignment

### Phase 3: Integration (TODO)
- [ ] Pass inference to codegen
- [ ] Update generate_literal() to use inference
- [ ] Add backend-specific suffix logic

### Phase 4: Advanced Constraints (TODO)
- [ ] Function call argument → parameter type
- [ ] Assignment target type propagation
- [ ] Local variable type tracking
- [ ] Cross-module inference

### Phase 5: Error Messages (TODO)
- [ ] Mixing error with source locations
- [ ] Helpful suggestions (casts, annotations)
- [ ] Multi-span errors (show both sides)

---

## Success Criteria

### Compiler Tests
- [ ] All 5 TDD tests passing
- [ ] No E0689 errors (ambiguous numeric type)
- [ ] No E0277 errors (f32/f64 mixing)
- [ ] Clear Windjammer errors (not Rust errors)

### Game Compilation
- [ ] Breach Protocol compiles cleanly
- [ ] No float-related errors
- [ ] Consistent types throughout

### Multi-Backend
- [ ] Rust: Uses suffixes
- [ ] Go: No suffixes
- [ ] JS: No suffixes
- [ ] Interpreter: No suffixes

---

## Current Status

**Tests:** 4/5 passing ✅
**Module:** Compiles successfully ✅
**Integration:** Not started ⏳
**Next:** Implement function parameter constraints to pass the final test

---

## Philosophy Alignment

**"Compiler Does the Hard Work, Not the Developer"** ✅
- Automatic float type inference
- No manual `_f32`/`_f64` annotations required
- Clear errors when mixing occurs

**"80% of Rust's power with 20% of Rust's complexity"** ✅
- Memory safety: ✅ (from Rust)
- Type safety: ✅ (enhanced with inference)
- Boilerplate: ❌ (compiler infers)

**"No Workarounds, Only Proper Fixes"** ✅
- Proper constraint-based type system
- Not a hack or backend-specific workaround
- Foundation for future type inference

**"Windjammer is NOT Rust Lite"** ✅
- Rust requires explicit type annotations
- Windjammer infers automatically
- Deviation serves our values

---

**Remember:** This is proper language design, not a quick fix. We're building for decades, not days.
