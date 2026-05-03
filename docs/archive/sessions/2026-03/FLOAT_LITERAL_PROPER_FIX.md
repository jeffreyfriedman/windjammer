# Proper Fix for Float Literal Type Mismatches

## The Problem

Breach Protocol has f32/f64 type mismatches in generated code:
```rust
let x: f32 = 2.0;  // ERROR: 2.0 defaults to f64
let y = pos.x + 10.0;  // ERROR: pos.x is f32, 10.0 is f64
```

## ❌ WRONG FIX (Attempted 2026-03-16, Reverted)

**What I did:** Changed compiler default from f64 to f32 "for game engines"

**Why it was wrong:**
- Violates domain-agnostic principle
- Biases compiler toward games over other domains
- Breaks existing code expecting f64 defaults
- Changes language semantics for application convenience

**User feedback:** "Never alter the language compiler for the convenience of the game engine!"

## ✅ CORRECT FIXES (In Priority Order)

### Option 1: Improve Type Inference (BEST)

Make inference more accurate by propagating types through expressions:

```rust
// In analyzer
fn infer_binary_op(&mut self, left: &Expr, op: BinOp, right: &Expr) {
    let left_type = self.infer_expr(left);
    let right_type = self.infer_expr(right);
    
    // If left is f32 and right is literal, infer right as f32
    if left_type == Type::F32 && matches!(right, Expr::Literal(Literal::Float(_))) {
        self.constrain_literal(right, Type::F32);
    }
}
```

**Pros:**
- Language-agnostic (works for all domains)
- No breaking changes
- Matches user intent naturally

**Cons:**
- More complex inference engine
- Needs thorough testing

### Option 2: Explicit Type Annotations (SIMPLEST)

Require users to annotate ambiguous literals:

```windjammer
// In Windjammer source
let x: f32 = 2.0_f32  // Explicit suffix
let y = pos.x + 10.0_f32  // Explicit suffix
```

**Pros:**
- Clear intent, no ambiguity
- No compiler changes needed
- Works immediately

**Cons:**
- More verbose
- Users must remember to annotate

### Option 3: Project-Level Configuration (FUTURE)

Add `.windjammer.toml` for project-wide settings:

```toml
[defaults]
float_literal_type = "f32"  # Project-wide default
```

**Pros:**
- Project chooses, not compiler
- Game projects can opt-in to f32
- Other projects unaffected

**Cons:**
- Requires implementing config system
- May lead to portability issues

### Option 4: Domain-Specific Plugin (CLEANEST)

`wj-game` plugin provides domain-specific helpers:

```bash
# wj-game adds post-processing to generated code
wj game build --float-literals=f32
```

Plugin rewrites generated Rust:
```rust
// Before (from compiler)
let x = 2.0;

// After (wj-game post-process)
let x = 2.0_f32;
```

**Pros:**
- Core compiler stays pure
- Game projects get convenience
- Other domains unaffected
- Explicit opt-in

**Cons:**
- Plugin complexity
- Debugging is harder (generated code differs from compiler output)

## Recommendation

**Short-term:** Option 2 (Explicit annotations)
- Immediate fix
- No compiler changes
- Clear and correct

**Long-term:** Option 1 (Better inference)
- Proper language improvement
- Benefits all domains
- Matches Rust's inference behavior

**NOT recommended:** Option 3 or 4
- Adds complexity
- May cause portability issues
- Better to fix inference properly

## Implementation Plan

### Phase 1: Immediate (Manual annotations)
```windjammer
// Add _f32 suffix to Breach Protocol source
let speed: f32 = 10.0_f32
let delta = self.velocity.x * dt_f32
```

### Phase 2: Compiler improvement (Type inference)
1. Write tests for binary op inference
2. Implement literal constraint propagation
3. Test with Breach Protocol
4. Verify no regressions in other projects

### Phase 3: Documentation
- Document best practices for numeric literals
- Add linter suggestion: "Consider explicit type suffix"
- Update Windjammer book with examples

## Lesson Learned

**The compiler is a language tool, not an application framework.**

When facing application-specific issues:
1. Ask: "Is this a language problem or application problem?"
2. Ask: "Would this benefit all users or just one domain?"
3. Ask: "Am I changing the language or fixing a bug?"

If the answer is "just one domain" → **It doesn't belong in the compiler.**

Domain-specific needs belong in:
- Plugins (wj-game, wj-web, wj-kernel)
- Project configuration
- Application-level tooling

**Never change language defaults for application convenience. Ever.**

---

*Written 2026-03-16 after reverting commit ea2ff5be*
*User feedback: "I have caught you in this shitty and lazy decision several times"*
*This must never happen again.*
