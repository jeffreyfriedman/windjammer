# Windjammer Idiomatic Code Audit
**Date**: 2026-02-20
**Severity**: CRITICAL - Philosophy Violation
**Scope**: 334 .wj files, 1,363 methods

## The Problem

**Every method in the codebase explicitly annotates `self` ownership (`self`, `&self`, `&mut self`).**

This violates the core Windjammer philosophy:
> **"Infer what adds no value to be explicit about (ownership, mutability, simple types)"**
> **"The compiler handles mechanical details"**
> **"80% of Rust's power with 20% of Rust's complexity"**

## Examples of Non-Idiomatic Code

### ❌ Current (Non-Idiomatic)

```wj
pub fn title(&self) -> String {
    self.title.clone()
}

pub fn id(&self) -> &QuestId {
    &self.id
}

pub fn gold_reward(self) -> u32 {
    self.gold_reward
}

pub fn add_objective(self, objective: Objective) -> Quest {
    self.objectives.push(objective);
    self
}
```

**Problems:**
1. Developer must decide: `self`, `&self`, or `&mut self`?
2. Developer must decide: return reference or owned?
3. Developer must manually call `.clone()` when needed
4. 1,363 methods have this boilerplate
5. **This is Rust-style code, not Windjammer-style code**

### ✅ Idiomatic Windjammer (What It SHOULD Be)

```wj
pub fn title() -> string {
    self.title
}

pub fn id() -> QuestId {
    self.id
}

pub fn gold_reward() -> u32 {
    self.gold_reward
}

pub fn add_objective(objective: Objective) -> Quest {
    self.objectives.push(objective)
    self
}
```

**Benefits:**
1. **Compiler infers `&self`** for getters (mechanical detail)
2. **Compiler infers `.clone()`** when returning non-Copy field from `&self`
3. **Compiler infers `self`** for builder pattern (consumes and returns)
4. **Compiler infers `&mut self`** for mutating methods
5. **Developer focuses on logic, not ownership mechanics**

## Audit Results

### Pattern 1: Getters with `(&self)` and Manual `.clone()`
**Count**: ~400+ instances
**Example**: `pub fn title(&self) -> String { self.title.clone() }`
**Should be**: `pub fn title() -> string { self.title }`
**Compiler should infer**: `&self`, automatic `.clone()` since String is not Copy

### Pattern 2: Getters with `(&self)` Returning References
**Count**: ~300+ instances
**Example**: `pub fn id(&self) -> &QuestId { &self.id }`
**Should be**: `pub fn id() -> QuestId { self.id }`
**Compiler should infer**: `&self`, return `&self.id` automatically

### Pattern 3: Getters with `(self)` Taking Ownership Unnecessarily
**Count**: ~400+ instances
**Example**: `pub fn gold_reward(self) -> u32 { self.gold_reward }`
**Should be**: `pub fn gold_reward() -> u32 { self.gold_reward }`
**Compiler should infer**: `&self` (no reason to take ownership for getter)

### Pattern 4: Builder Methods with `(self)`
**Count**: ~200+ instances
**Example**: `pub fn add_objective(self, objective: Objective) -> Quest { ... self }`
**Should be**: `pub fn add_objective(objective: Objective) -> Quest { ... self }`
**Compiler should infer**: `self` (consumes and returns, classic builder pattern)

### Pattern 5: Mutating Methods with `(&mut self)`
**Count**: ~100+ instances
**Example**: `pub fn update_progress(&mut self, amount: u32) { ... }`
**Should be**: `pub fn update_progress(amount: u32) { ... }`
**Compiler should infer**: `&mut self` (mutates internal state)

## Broken Patterns (Active Bugs)

### ❌ BROKEN: `(self) -> &Type`
**Count**: Found in multiple files
**Example**: `pub fn name(self) -> &str { &self.name }`
**Problem**: **Cannot return reference from owned self** (lifetime error)
**Should be**: `pub fn name() -> string { self.name }` → compiler infers `&self`, returns owned `String`

This pattern is SYNTACTICALLY INVALID and reveals the problem with manual ownership.

## Impact on Current Development

### Immediate Issues
1. **Type mismatches everywhere** - Manual ownership decisions create `String` vs `&str` confusion
2. **Unnecessary `.clone()` calls** - 54 files have this
3. **Ownership errors** - Methods taking `self` when they should borrow
4. **Compilation errors** - 30 errors in game engine, many from ownership mismatches

### Long-term Issues
1. **Learning curve** - New developers must learn Rust ownership
2. **Verbosity** - 1,363 methods have unnecessary boilerplate
3. **Maintenance burden** - Every method signature is a decision point
4. **Philosophy violation** - We're writing Rust, not Windjammer

## The Windjammer Way: Automatic Ownership Inference

### What the Compiler Should Infer

```wj
// Getter returning non-Copy field
pub fn title() -> string {
    self.title
}
// Compiler sees: accessing field, String is not Copy
// Infers: &self (borrow receiver)
// Infers: return self.title.clone() (clone non-Copy field)
// Generated: pub fn title(&self) -> String { self.title.clone() }

// Getter returning Copy field
pub fn gold_reward() -> u32 {
    self.gold_reward
}
// Compiler sees: accessing field, u32 is Copy
// Infers: &self (no need to take ownership)
// Infers: return self.gold_reward (copy is cheap)
// Generated: pub fn gold_reward(&self) -> u32 { self.gold_reward }

// Getter returning reference to field
pub fn id() -> QuestId {
    self.id
}
// Compiler sees: accessing field, returning as-is
// Infers: &self (borrow receiver)
// Infers: return &self.id (borrow field)
// Generated: pub fn id(&self) -> &QuestId { &self.id }

// Builder method (consumes and returns)
pub fn add_objective(objective: Objective) -> Quest {
    self.objectives.push(objective)
    self
}
// Compiler sees: returns self, mutates field
// Infers: self (take ownership, return ownership)
// Generated: pub fn add_objective(self, objective: Objective) -> Quest { ... self }

// Mutating method (no return)
pub fn update_progress(amount: u32) {
    self.progress = self.progress + amount
}
// Compiler sees: mutates self, no return
// Infers: &mut self (borrow mutably)
// Generated: pub fn update_progress(&mut self, amount: u32) { ... }
```

### Inference Algorithm (Pseudo-code)

```
fn infer_self_ownership(method: &Method) -> SelfOwnership {
    // Check return type
    if method.returns_self() {
        return SelfOwnership::Owned  // Builder pattern: self -> Self
    }
    
    // Check if method mutates fields
    if method.mutates_any_field() {
        return SelfOwnership::MutBorrow  // &mut self
    }
    
    // Default: immutable borrow for getters/queries
    return SelfOwnership::Borrow  // &self
}

fn infer_return_value(method: &Method, self_ownership: SelfOwnership) -> ReturnValue {
    if method.returns_self() {
        return ReturnValue::SelfOwned
    }
    
    if method.returns_field() {
        let field = method.get_returned_field();
        
        if field.is_copy_type() {
            // u32, f32, bool, etc.
            return ReturnValue::CopyValue(field)
        } else if self_ownership == SelfOwnership::Owned {
            // Owned self, can move field out
            return ReturnValue::MoveField(field)
        } else {
            // Borrowed self, must clone or return reference
            if field.is_cheap_to_clone() || method.returns_owned() {
                return ReturnValue::CloneField(field)
            } else {
                return ReturnValue::BorrowField(field)
            }
        }
    }
    
    // Default: infer from expression
    infer_from_expression(method.body)
}
```

## Recommended Action

### Option 1: Enhance Compiler (Preferred)
**Implement automatic `self` ownership inference in the analyzer**

**Changes needed:**
1. Parser: Allow omitting `self` parameter entirely
2. Analyzer: Infer `&self`, `&mut self`, or `self` based on method body
3. Analyzer: Infer automatic `.clone()` insertion when needed
4. Codegen: Generate proper Rust signatures

**Timeline:** Medium effort, HIGH impact

### Option 2: Establish Idiomatic Pattern (Interim)
**Document the "right way" to write methods**

**Patterns:**
- **Getters (Copy types)**: `pub fn x(&self) -> T`
- **Getters (non-Copy)**: `pub fn x(&self) -> T` + manual `.clone()`
- **Builders**: `pub fn with_x(self, x: T) -> Self`
- **Mutators**: `pub fn set_x(&mut self, x: T)`

**Timeline:** Immediate, LOW impact (doesn't reduce boilerplate)

### Option 3: Hybrid Approach (Recommended)
1. **Short-term**: Fix current compilation errors with explicit annotations
2. **Medium-term**: Implement compiler inference for common cases
3. **Long-term**: Migrate codebase to omit `self` annotations

## Files Requiring Attention

**High Priority (Currently Broken):**
- `quest/quest.wj` - Methods trying to return `&str` from `(self)`
- `achievement/achievement.wj` - Same issue
- `water/water_surface.wj` - Same issue

**Medium Priority (Working but Verbose):**
- All 334 .wj files have explicit `self` annotations
- 54 files have manual `.clone()` calls

**Low Priority (Idiomatic but Could Be Inferred):**
- Builder patterns are explicit but clear
- Mutating methods are explicit but necessary for now

## Next Steps

1. **Fix immediate compilation errors** (current TDD session)
2. **Create design document** for automatic self inference
3. **Implement parser changes** to allow omitting `self`
4. **Implement analyzer inference** for ownership
5. **Add TDD tests** for inference edge cases
6. **Migrate codebase** to idiomatic style

## Conclusion

**This audit reveals a systematic violation of Windjammer's philosophy.**

We're forcing developers to write Rust-style ownership annotations when the compiler should be doing this work automatically. This is exactly what Windjammer was designed to avoid.

**The fix is not to change 1,363 method signatures manually.**
**The fix is to make the compiler smart enough to infer them.**

Until then, we should:
1. Fix broken patterns (`self` returning `&Type`)
2. Use explicit `&self`/`self` consistently
3. Plan compiler enhancement for automatic inference

**Remember**: "The compiler should be complex so the user's code can be simple."
