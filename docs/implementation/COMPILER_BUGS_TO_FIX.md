# Compiler Bugs Found via Dogfooding - TO FIX WITH TDD

## ✅ FIXED BUGS

## Bug #6: Enum match on `&self` creates borrowed bindings needing deref [FIXED ✅]

**Status:** ✅ FIXED - 2026-02-26
**Test Case:** `tests/bug_enum_self_borrow.wj` (PASSING)
**Severity:** High (blocked dialogue system compilation)

**Problem:**
When matching on `&self` inside impl methods, enum variant destructuring creates borrowed bindings (`&T`). These weren't being auto-dereferenced in comparisons.

**Example:**
```windjammer
impl Condition {
    pub fn check(self) -> bool {  // codegen converts to &self
        match self {
            Condition::ThresholdCheck(threshold) => {
                return get_value() > threshold;  // ERROR: expected i32, found &i32
            }
        }
    }
}
```

**Root Cause:** `match_expression_binds_refs()` didn't check if the matched expression is a borrowed parameter (like `&self`), so `borrowed_iterator_vars` wasn't populated.

**Fix:**
- Added `Expression::Identifier` case to `match_expression_binds_refs`
- Check if identifier is in `inferred_borrowed_params`
- This correctly populates `borrowed_iterator_vars` with enum bindings
- Existing Bug #5 deref logic then applies auto-deref in comparisons

**Generated Code (After Fix):**
```rust
return get_value() > *threshold;  // ✅ Auto-deref applied!
```

**Impact:** Dialogue system enum conditions now compile correctly.

---

## Bug #1: Method self-by-value incorrectly infers &mut [FIXED ✅]

**Status**: ✅ FIXED - Test passing as of 2026-02-25

**Discovered**: 2026-02-24 during camera matrices test compilation

**Symptom**:
```windjammer
impl Mat4 {
    fn multiply(self, other: Mat4) -> Mat4 { ... }
}

fn test() {
    let identity = Mat4::identity()  // Compiler says: needs 'mut'
    let result = identity.multiply(other)  // Error: cannot borrow as mutable
}
```

**Root Cause**:
In `windjammer/src/analyzer.rs` lines 937-981, when `param.ownership == OwnershipHint::Owned` (user wrote `self` not `&self`), the analyzer incorrectly checks if the method modifies fields and downgrades to `OwnershipMode::MutBorrowed` (&mut self).

**The Fix**:
```rust
// CURRENT (BUGGY):
OwnershipHint::Owned => {
    if param.name == "self" {
        let modifies_fields = self.function_modifies_self_fields(func);
        if modifies_fields {
            OwnershipMode::MutBorrowed  // ❌ WRONG!
        } else {
            OwnershipMode::Owned
        }
    } else {
        OwnershipMode::Owned
    }
}

// SHOULD BE:
OwnershipHint::Owned => {
    // When user explicitly writes `self` (Owned), RESPECT IT!
    // Don't analyze or downgrade. User wants owned.
    OwnershipMode::Owned
}
```

**Test Case**: `windjammer/tests/method_self_by_value.wj`

**Impact**: Blocks clean implementation of math libraries (Mat4, Vec3, etc.) that use self-by-value for transforms.

**Workaround**: Mark variables as `mut` even though not needed, or use `&self` instead of `self`.

**Update (2026-02-24)**: Fixed parameter inference (analyzer.rs:937-943) ✅  
**Remaining**: Method call site still infers &mut for receiver. Need to trace codegen.

**Priority**: HIGH - Common pattern in game math

**Next Steps**: 
1. Find where method call receivers get mutability inference
2. Check method signature to see if it takes `self` vs `&mut self`
3. Don't add `&mut` if method takes `self` by value

---

## Bug #2: format! in temp variable generates &_temp instead of _temp [FIXED ✅]

**Status**: ✅ FIXED - Verified in game library as of 2026-02-25

**Discovered**: 2026-02-25 during assets/loader.wj compilation

**Symptom**:
```windjammer
enum AssetError {
    InvalidFormat(String),
}

fn validate() -> Result<(), AssetError> {
    Err(AssetError::InvalidFormat(format!("Error: {}", code)))
}
```

**Generated Rust** (BUGGY):
```rust
Err({ let _temp0 = format!("Error: {}", code); AssetError::InvalidFormat(&_temp0) })
//                                                                        ^^^^^^^^ BUG!
```

**Root Cause**:
In `windjammer/src/codegen/rust/generator.rs`, when generating code for `format!()` in an expression context, the compiler creates a temporary variable but incorrectly passes `&_temp0` instead of `_temp0`.

This causes two problems:
1. **Type mismatch**: Expected `String`, found `&String`
2. **Lifetime error**: `_temp0` goes out of scope immediately

**The Fix**:
```rust
// Should generate:
Err(AssetError::InvalidFormat(format!("Error: {}", code)))

// Or if temp is needed:
Err({ let _temp0 = format!("Error: {}", code); AssetError::InvalidFormat(_temp0) })
//                                                                         ^^^^^^^^ No &!
```

**Test Case**: `windjammer/tests/bug_format_temp_var_lifetime.wj`

**Impact**: Blocks any use of `format!()` in enum variants, function args, or struct fields that expect `String`.

**Patterns Affected**:
- `Err(EnumVariant(format!(...)))`
- `func_call(format!(...))`
- `Struct { field: format!(...) }`

**Priority**: HIGH - Common pattern in error handling

**Next Steps**:
1. Find where format! generates temp variables in codegen
2. Check if the result is being borrowed when it shouldn't be
3. Remove the `&` prefix or eliminate temp variable entirely

---

## Bug #3: While-loop index incorrectly inferred as i64 instead of usize [HIGH PRIORITY]

**Status**: ✅ FIXED (2026-02-26 01:15 PST) - TDD test passing, game library compiles

**Discovered**: 2026-02-25 during animation/clip.wj compilation
**Fixed**: 2026-02-26 01:15 PST (usize_variables now persists during statement generation)

---

## Bug #4: Array indexing with expression (i + 1) incorrectly typed as i64 [HIGH PRIORITY]

**Status**: ✅ FIXED (2026-02-26 01:40 PST) - TDD test passing, game library compiles

**Discovered**: 2026-02-26 01:20 PST during animation/clip.wj compilation (dogfooding session)
**Fixed**: 2026-02-26 01:40 PST (expression_produces_usize() now handles Binary expressions properly)
**Test Case**: `tests/bug_array_index_expression_type.wj` ✅ PASSING

---

## Bug #5: Parameter ownership not inferred for type aliases / newtype wrappers

**Status**: 🔴 ACTIVE - TDD test created, investigating fix

**Discovered**: 2026-02-26 01:45 PST during dialogue module compilation (dogfooding session)
**Test Case**: `tests/bug_newtype_wrapper_inference.wj` ✅ CREATED

**Symptom**:
```rust
error[E0308]: mismatched types
  --> bug_newtype_wrapper_inference.rs:29:33
   |
29 |         self.is_quest_completed(quest_id)
   |              ------------------ ^^^^^^^^ expected `String`, found `&String`
```

**Windjammer Code**:
```windjammer
type QuestId = String

fn is_quest_completed(&self, quest_id: QuestId) -> bool {
    self.completed_quests[i] == quest_id  // Only reads quest_id
}

fn check_with_ref(&self, quest_id: &QuestId) -> bool {
    self.is_quest_completed(quest_id)  // ERROR: expected QuestId, found &QuestId
}
```

**Generated Rust** (BUGGY):
```rust
fn is_quest_completed(&self, quest_id: QuestId) -> bool {  // Owned
    self.completed_quests[i] == quest_id
}
```

**Expected Rust**:
```rust
fn is_quest_completed(&self, quest_id: &QuestId) -> bool {  // Borrowed
    self.completed_quests[i] == quest_id
}
```

**Root Cause**:
Ownership inference doesn't recognize that `quest_id` should be `&QuestId` because:
1. It's only used in comparisons (read-only)
2. Type aliases (like `QuestId = String`) should follow same rules as the underlying type
3. Parameter is passed by reference at call sites

**Fix Strategy**:
Update parameter ownership inference in analyzer to treat type alias parameters same as their underlying types for ownership analysis.

**Symptom**:
```windjammer
let mut after_idx = keyframes.len() - 1  // usize
for i in 0..keyframes.len() {
    after_idx = i + 1  // Error: expected usize, found i64
}
```

**Generated Rust** (BUGGY):
```rust
let mut after_idx = self.keyframes.len() - 1;  // usize
let mut i = 0;
while i < ((self.keyframes.len() - 1) as i64) {  // BUG: i is i64!
    after_idx = i + 1;  // ERROR: expected usize, found i64
    i += 1;
}
```

**Root Cause**:
In `windjammer/src/codegen/rust/generator.rs`, when converting for-loops to while-loops, the compiler:
1. Defaults loop index `i` to `i64`
2. Casts `.len()` to `i64` for comparison
3. SHOULD infer `i` as `usize` when:
   - Loop bound is `.len()` (which is usize)
   - Index assigned to usize variable
   - Index used for array indexing

**The Fix**:
Improve type inference for loop indices:
1. Check if loop bound is `.len()` or other usize expression
2. Check if index is used with usize variables/indexing
3. Infer `i` as `usize` instead of defaulting to `i64`

**Test Case**: `windjammer/tests/bug_loop_index_usize_inference.wj`

**Impact**: Blocks any pattern where loop index is assigned to usize variables (common in animation, pathfinding, searching).

**Priority**: HIGH - Common pattern in game code

**Next Steps**:
1. Find loop index type inference logic in codegen
2. Add usize inference when bound is .len()
3. Propagate usize type through arithmetic (i + 1, i - 1)

---

## Future Bugs to Document Here

(Add more as we find them via dogfooding)

---

## The Windjammer Way

**"No workarounds, only proper fixes."**

Every bug found via dogfooding should:
1. Have a failing test case
2. Be documented here
3. Have the fix location identified  
4. Be fixed with TDD before shipping

This file is temporary - bugs should be fixed and removed, not accumulated!
