# Compiler Bugs Found via Dogfooding - TO FIX WITH TDD

## Bug #1: Method self-by-value incorrectly infers &mut [HIGH PRIORITY]

**Status**: 🔴 OPEN - Test exists, fix identified, needs implementation

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

**Priority**: HIGH - Common pattern in game math

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
