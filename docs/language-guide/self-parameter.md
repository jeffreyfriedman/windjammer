# The `self` Parameter

## Why `self` is Special

In Windjammer, the `self` parameter gets automatic ownership inference:

```windjammer
fn read(self) {
    println(self.x)
}

fn update(self) {
    self.x = 1
}
```

The compiler infers:
- `read` → `&self` (no mutation)
- `update` → `&mut self` (mutation detected)

## Why This Makes Sense

1. **Not user-declared** — `self` is implicitly provided by the language
2. **Always present** — Every method has it
3. **Mechanical detail** — `&` vs `&mut` determined by usage, not intent

## Contrast with Variables

```windjammer
let x = 0          // Immutable (default)
let mut y = 0      // Mutable (explicit)
```

**Variables require explicit `mut` because:**
- User controls when variables can change
- Prevents accidental mutations
- Documents business logic intent

**Parameters infer ownership because:**
- Compiler determines optimal passing mechanism
- Reduces boilerplate
- Implementation detail, not business logic

## Don't Write `mut self`

```windjammer
// ❌ WRONG - mut is not needed
fn update(mut self) {
    self.x = 1
}

// ✅ CORRECT - compiler infers &mut self
fn update(self) {
    self.x = 1
}
```

Windjammer infers ownership automatically. Writing `mut self` is redundant and will produce an error.

## Same Rule for Other Parameters

Ownership inference applies to **all** parameters, not just `self`:

```windjammer
fn process(item: Item) {
    item.x = 42    // Compiler infers: &mut Item
}

fn read_only(data: Data) {
    println(data.value)  // Compiler infers: &Data
}
```

## Summary

| Concept | Explicit or Inferred? | Why? |
|---------|----------------------|------|
| Variable mutability (`let mut`) | **Explicit** | User intent |
| Parameter ownership (`&`, `&mut`) | **Inferred** | Compiler optimization |

**These are orthogonal concerns.** Windjammer keeps them separate for clarity and safety.
