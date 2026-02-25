# Compiler Bugs Found via Dogfooding - TO FIX WITH TDD

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

## Bug #2: format! in temp variable generates &_temp instead of _temp [HIGH PRIORITY]

**Status**: 🔴 ACTIVE - TDD test created, fix in progress

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
