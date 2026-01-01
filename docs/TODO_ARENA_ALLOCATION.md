# TODO: Arena Allocation for AST

## Problem

The compiler currently uses `Box<T>` for AST nodes, which causes:
1. **Deep Drop recursion** - Dropping a 1000-node AST = 1000 recursive drop calls
2. **Stack overflow on Windows** - Currently requiring 64MB stack (unsustainable)
3. **Poor cache locality** - Each node allocated separately on heap
4. **Slower allocation** - Individual `Box::new()` calls for each node
5. **Memory overhead** - Per-node Box allocations have overhead

## Solution: Arena Allocation

### Industry Standard
- **rustc** uses `typed-arena` for AST
- **clang** uses bump allocator
- **LLVM** uses arena allocation
- **Every major compiler** uses this pattern

### Benefits
- ✅ **No Drop recursion** - Deallocate entire arena at once
- ✅ **Better cache locality** - Contiguous memory layout
- ✅ **Faster allocation** - Bump allocator (just increment pointer)
- ✅ **Smaller memory footprint** - No per-node overhead
- ✅ **Normal stack size** - No need for 64MB stack

### Implementation Plan

#### Phase 1: Add typed-arena dependency
```toml
[dependencies]
typed-arena = "2.0"
```

#### Phase 2: Add arena to compiler struct
```rust
use typed_arena::Arena;

pub struct Compiler<'ast> {
    // All AST nodes allocated from this arena
    ast_arena: &'ast Arena<parser::Expression>,
    stmt_arena: &'ast Arena<parser::Statement>,
    // ...other arenas for different node types
}
```

#### Phase 3: Change AST to use references
```rust
// Before: Owned nodes
pub enum Expression {
    Binary {
        left: Box<Expression>,  // ← Owned, causes Drop recursion
        op: BinaryOp,
        right: Box<Expression>, // ← Owned, causes Drop recursion
    },
}

// After: Borrowed nodes
pub enum Expression<'ast> {
    Binary {
        left: &'ast Expression<'ast>,  // ← Borrowed from arena
        op: BinaryOp,
        right: &'ast Expression<'ast>, // ← Borrowed from arena
    },
}
```

#### Phase 4: Update allocation sites
```rust
// Before
let expr = Box::new(Expression::Binary { ... });

// After
let expr = arena.alloc(Expression::Binary { ... });
```

#### Phase 5: Remove manual Drop implementations
```rust
// No longer needed - arena drops everything at once!
// impl Drop for Expression { ... } ← DELETE
```

### Example: How rustc Does It

```rust
// From rustc: src/librustc_ast/ast.rs
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
    // No Box! All sub-expressions are &'ast Expr
}

pub enum ExprKind {
    Binary {
        left: &'ast Expr,   // ← Reference, not Box
        op: BinOp,
        right: &'ast Expr,  // ← Reference, not Box
    },
}

// Allocated from arena:
impl<'ast> AstBuilder<'ast> {
    fn alloc_expr(&self, kind: ExprKind) -> &'ast Expr {
        self.arena.alloc(Expr {
            id: self.next_id(),
            kind,
            span: DUMMY_SP,
        })
    }
}
```

### Migration Strategy

1. **Start with new code** - Use arena for new features
2. **Migrate incrementally** - One AST type at a time
3. **Keep compatibility** - Both patterns can coexist during migration
4. **Test thoroughly** - Ensure no lifetime issues

### Performance Expectations

Based on rustc benchmarks:
- **2-3x faster** compilation (better allocation)
- **30-40% less memory** usage (no Box overhead)
- **Normal stack size** (2-8MB instead of 64MB)
- **No stack overflows** (no Drop recursion)

## Priority

**HIGH** - The 64MB stack workaround is not sustainable. This should be done soon after CI is green.

## References

- rustc arena: https://github.com/rust-lang/rust/tree/master/compiler/rustc_arena
- typed-arena crate: https://docs.rs/typed-arena
- LLVM arena: https://llvm.org/doxygen/classllvm_1_1BumpPtrAllocator.html
- Compiler Design Patterns: https://rust-lang.github.io/rustc-dev-guide/memory.html

## Related Issues

- Windows stack overflow (needs 64MB)
- Slow compilation times
- High memory usage

---

**Action Item:** Create follow-up PR after CI is green: `feat: Use arena allocation for AST to improve performance and eliminate Drop recursion`

