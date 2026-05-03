# Language Soundness & Consistency Audit: 2026-03-14

## Question: Is Automatic `self` Inference Inconsistent?

**User concern:** "When declaring variables, we assume immutability unless mut is specified. Do the improvements above align with that?"

---

## Current Behavior

### Variables (EXPLICIT mutability)
```windjammer
let x = 0          // Immutable (default)
let mut y = 0      // Mutable (explicit)

x = 1              // ERROR: cannot assign to immutable variable
y = 1              // OK: y is declared mut
```

### Method Parameters (IMPLICIT ownership)
```windjammer
fn read_value(self) {
    println(self.value)     // Compiler infers: &self (no mutation)
}

fn update_value(self) {
    self.value = 42         // Compiler infers: &mut self (mutation detected)
}
```

---

## Question: Is This Inconsistent?

### User's Valid Concern:
- Variables require **explicit** `mut`
- Method parameters have **automatic** mutability inference
- **Seems inconsistent?**

---

## Analysis: Two Different Concepts

### 1. **Variable Mutability** (User Control)

**Purpose:** Prevent accidental mutations in business logic

```windjammer
let count = 0
// ... 100 lines of code ...
count = 1  // ERROR - prevents accidental bug!
```

**Why explicit:** This is a **semantic decision** the developer makes:
- "Should this variable change?"
- "Is mutation part of my algorithm?"
- **Business logic concern**

**Value of explicitness:**
- Documents intent
- Prevents accidental reassignment
- Aids debugging (mutations are visible)

---

### 2. **Parameter Ownership** (Compiler Optimization)

**Purpose:** Determine `&`, `&mut`, or owned for function parameters

```windjammer
fn process(item: Item) {  // Is this &Item, &mut Item, or Item?
    // Compiler analyzes usage to decide
}
```

**Why implicit:** This is a **mechanical detail** the compiler determines:
- "Does this parameter get mutated?"
- "Does this parameter get moved?"
- **Implementation detail**

**Value of implicitness:**
- Reduces boilerplate (`&`, `&mut` everywhere)
- Compiler can't get it wrong (analyzes actual usage)
- Refactoring-friendly (change implementation, signature auto-updates)

---

## The "`self` is Special" Argument

### Key Insight: `self` is NOT a User Variable

```windjammer
// User declares a variable:
let mut x = 0       // ✅ User controls mutability

// User does NOT declare self:
fn update(self) {   // ❌ self is compiler-managed
    self.x = 1
}
```

**Why `self` is different:**
1. **Not user-declared** - `self` is implicitly provided by the language
2. **Always present** - Every method has `self` (unlike user variables)
3. **Mechanical detail** - Whether it's `&self` or `&mut self` is determined by usage, not intent

---

## Consistency Check: Windjammer Philosophy

### Principle: "Infer What Doesn't Matter, Explicit Where It Does"

| Aspect | Explicit or Inferred? | Why? | Philosophy |
|--------|----------------------|------|------------|
| **Variable mutability** | ✅ **EXPLICIT** (`let mut`) | Business logic decision | "Explicit where it matters" |
| **Parameter ownership** | ✅ **INFERRED** (`&`, `&mut`, owned) | Mechanical detail | "Infer what doesn't matter" |
| **Generic types** | ✅ **INFERRED** (in bodies) | Mechanical detail | "Infer what doesn't matter" |
| **Function return types** | ⚠️ **EXPLICIT** (signatures) | API contract | "Explicit where it matters" |

**Result: CONSISTENT!** ✅

---

## Comparison: Rust vs Windjammer

### Rust: Explicit Everywhere

```rust
fn update(&mut self) {      // Must write &mut
    self.x = 1;
}

let mut v = Vec::new();     // Must write mut
v.push(1);
```

**Philosophy:** "Be explicit about everything"  
**Result:** Verbose but consistent

---

### Windjammer: Explicit Intent, Inferred Details

```windjammer
fn update(self) {           // Compiler infers &mut from usage
    self.x = 1
}

let mut v = Vec::new()      // Must write mut (user intent)
v.push(1)
```

**Philosophy:** "Explicit intent, automatic details"  
**Result:** Concise and safe

---

## Potential Confusion Points

### ❌ BAD: If `self` mutation required `mut`

```windjammer
fn update(self) {
    // Does this require `mut self`? Confusing!
    self.x = 1
}
```

**Problem:** `mut self` would be redundant - we're already detecting mutation!

---

### ✅ GOOD: Current behavior

```windjammer
fn update(self) {
    self.x = 1              // Compiler: "I see mutation, use &mut self"
}

fn read(self) {
    println(self.x)         // Compiler: "No mutation, use &self"
}
```

**Benefit:** Developer writes intent, compiler handles details!

---

## Edge Cases to Consider

### 1. **Other Parameters (Not `self`)**

**Current behavior:**
```windjammer
fn add_to_list(list: Vec<int>, item: int) {
    list.push(item)         // Does compiler infer &mut for list?
}
```

**Question:** Should non-`self` parameters also get automatic ownership inference?

**Answer:** ✅ **YES** - Same principle applies!
- If `list.push()` is called, compiler infers `&mut Vec<int>`
- If `list` is only read, compiler infers `&Vec<int>`
- If `list` is consumed, compiler infers owned `Vec<int>`

**Consistency:** All parameters get automatic ownership inference (not just `self`).

---

### 2. **Assignment vs Mutation**

```windjammer
// Local variable assignment (requires mut)
let mut x = 0
x = 1               // ✅ Requires mut - prevents accidental reassignment

// Parameter mutation (automatic)
fn update(item: Item) {
    item.field = 1  // ✅ Compiler infers &mut - prevents boilerplate
}
```

**Distinction:**
- **Assignment** (`x = 1`) - Rebinding variable (explicit `mut` required)
- **Mutation** (`item.field = 1`) - Modifying through reference (automatic inference)

**Consistent?** ✅ YES
- Assignment affects control flow → explicit
- Mutation is mechanical → inferred

---

## Rust Leakage Check

**Question:** Are we accidentally mimicking Rust's verbosity?

```rust
// Rust requires:
fn update(&mut self) { ... }
fn process(&mut item: &mut Item) { ... }
```

```windjammer
// Windjammer infers:
fn update(self) { ... }         // → &mut self
fn process(item: Item) { ... }  // → &mut Item
```

**Result:** ✅ NO LEAKAGE - We're successfully abstracting Rust's verbosity!

---

## Soundness Guarantees

### Does automatic inference break safety?

**NO - Compiler still enforces all borrow checking rules:**

```windjammer
fn example(self) {
    let x = self.field      // Compiler infers &self
    self.field = 42         // ERROR: Can't have &self and &mut self!
}
```

**Safety preserved:**
- ✅ No data races (borrow checker enforced)
- ✅ No use-after-free (lifetime checking)
- ✅ No null pointers (Option type)

**Automatic inference is purely a convenience - safety is identical to Rust!**

---

## Cross-Language Comparison

### Swift

```swift
func update() {             // No & or &mut needed
    self.x = 1              // Mutability inferred
}

var x = 0                   // Must write var for mutability
x = 1
```

**Swift approach:** Similar to Windjammer (infer method mutability, explicit variable mutability)

---

### Kotlin

```kotlin
fun update() {              // No & or &mut needed
    this.x = 1              // Mutability inferred
}

var x = 0                   // Must write var for mutability
x = 1
```

**Kotlin approach:** Similar to Windjammer

---

### Rust

```rust
fn update(&mut self) {      // Must write &mut
    self.x = 1;
}

let mut x = 0;              // Must write mut
x = 1;
```

**Rust approach:** Explicit everywhere (consistent but verbose)

---

## Conclusion: Is Windjammer Consistent?

### ✅ YES - Two Different Concepts

1. **Variable Mutability (Explicit)**
   - **What:** Can this variable be reassigned?
   - **Why explicit:** Business logic decision, prevents bugs
   - **Syntax:** `let mut x`

2. **Parameter Ownership (Inferred)**
   - **What:** Is this `&`, `&mut`, or owned?
   - **Why inferred:** Mechanical detail, reduces boilerplate
   - **Syntax:** `fn foo(self)` → compiler determines

**These are orthogonal concerns!**

---

## Recommendation: Document the Distinction

### Add to Language Guide:

#### "Mutability vs Ownership"

**Windjammer distinguishes two concepts:**

1. **Variable Mutability** - Can a variable be reassigned?
   ```windjammer
   let x = 0          // Immutable
   let mut y = 0      // Mutable
   y = 1              // OK
   ```

2. **Parameter Ownership** - How is a parameter passed?
   ```windjammer
   fn read(item: Item) {      // Compiler infers: &Item
       println(item.name)
   }
   
   fn update(item: Item) {    // Compiler infers: &mut Item
       item.name = "new"
   }
   ```

**Why separate?**
- **Mutability:** User decision (business logic)
- **Ownership:** Compiler decision (mechanical detail)

**This keeps Windjammer simple AND safe!**

---

## Action Items

### 1. ✅ Add Language Guide Section

**File:** `docs/language-guide/mutability-vs-ownership.md`

**Content:**
- Explain the distinction
- Provide examples
- Justify the design decision
- Compare to Rust/Swift/Kotlin

---

### 2. ✅ Add Compiler Error Message

When user tries `mut self`:

```
error: `mut` is not needed for method parameters
 --> example.wj:5:8
  |
5 | fn update(mut self) {
  |           ^^^ help: remove `mut`, ownership is inferred automatically
  |
  = note: Windjammer infers `&self`, `&mut self`, or owned based on usage
  = note: Use `let mut x` for local variable mutability
```

---

### 3. ✅ Audit All Parameter Inference

**Ensure consistency:**
- `self` → inferred
- Other parameters → inferred (SAME rule)
- Local variables → explicit `mut`

---

## Final Verdict

### Is automatic `self` inference consistent with explicit variable `mut`?

**YES! ✅**

**Reason:**
- **Variables:** User controls reassignment (explicit `mut`)
- **Parameters:** Compiler controls passing mechanism (inferred ownership)
- **Orthogonal concerns!**

**Windjammer's approach:**
- ✅ Safe (same guarantees as Rust)
- ✅ Simple (less boilerplate than Rust)
- ✅ Consistent (explicit intent, inferred details)

**Philosophy alignment:** 🎯 PERFECT

---

## Summary

| Concern | Status | Resolution |
|---------|--------|-----------|
| Inconsistent with variable `mut`? | ✅ **NO** | Different concepts (reassignment vs ownership) |
| Confusing to users? | ⚠️ **MAYBE** | Add documentation + clear error messages |
| Sound and safe? | ✅ **YES** | All borrow checking rules enforced |
| Aligned with philosophy? | ✅ **YES** | "Infer what doesn't matter" |

**Recommendation:** Keep current design, add comprehensive documentation!
