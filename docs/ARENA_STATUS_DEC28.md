# Arena Allocation - Status Update (Dec 28, 2025)

## Progress: ~30% Complete

### ✅ Completed (Commits 1b8d8780, 62284d63, 64f218c2)

1. **AST Core Types** - Added `'ast` lifetime parameter:
   - `Expression<'ast>`
   - `Statement<'ast>`
   - `MatchArm<'ast>`, `Decorator<'ast>`
   - `FunctionDecl<'ast>`, `StructDecl<'ast>`, `TraitDecl<'ast>`
   - `ImplBlock<'ast>`, `Item<'ast>`, `Program<'ast>`
   - Changed all `Box<Expression>` → `&'ast Expression<'ast>`

2. **Parser Infrastructure**:
   - Added `Arena<Expression>` and `Arena<Statement>` to Parser
   - `alloc_expr()` and `alloc_stmt()` helper methods (using unsafe transmute)
   - Updated `parse()` signature: `parse<'parser>(&'parser mut self) -> Result<Program<'parser>>`

3. **Analyzer** (Partial):
   - `AnalyzedFunction<'ast>`
   - `Analyzer<'ast>`  
   - `ProgramAnalysisResult<'ast>`

### 🔄 In Progress

**Current:** Systematically fixing 396 compilation errors

**Strategy:** 
1. Fix core types first (Parser, Analyzer, CodeGenerator)
2. Then fix modules (expression_parser, statement_parser, etc.)
3. Then fix optimizer passes
4. Then fix tests
5. Finally reduce stack from 64MB → 8MB

### ❌ Remaining Work (70%)

#### Major Files To Update:

**Core Compiler:**
- [ ] `src/codegen/rust/generator.rs` (150+ methods, ~3000 lines)
- [ ] `src/analyzer.rs` (200+ methods that use AST, ~4000 lines)
- [ ] `src/parser_impl.rs` (all parse_* methods)
- [ ] `src/compiler_database.rs` (Salsa integration)

**Parser Modules:**
- [ ] `src/parser/expression_parser.rs` (90+ Expression references)
- [ ] `src/parser/statement_parser.rs`
- [ ] `src/parser/item_parser.rs`
- [ ] `src/parser/type_parser.rs`
- [ ] `src/parser/pattern_parser.rs`

**Codegen:**
- [ ] `src/codegen/rust/*.rs` (15+ files)
- [ ] `src/codegen/javascript/*.rs` (4 files)

**Optimizer:**
- [ ] `src/optimizer/*.rs` (15 phase files)
- [ ] `src/auto_clone.rs`
- [ ] `src/inference.rs`

**Other:**
- [ ] `src/errors/*.rs`
- [ ] `src/ejector.rs`
- [ ] `src/module_system.rs`
- [ ] All test files (28 files)

#### Allocation Sites:

**Pattern to replace (900+ locations):**
```rust
// Before
Box::new(Expression::Binary {
    left: Box::new(...),
    right: Box::new(...),
})

// After  
self.alloc_expr(Expression::Binary {
    left: self.alloc_expr(...),
    right: self.alloc_expr(...),
})
```

**Key Challenge:** Many allocations are nested 3-4 levels deep!

### 📊 Error Breakdown

**Current:** 396 errors

**Categories:**
1. **Missing lifetime specifiers** (~300 errors)
   - Structs containing AST types
   - Functions returning AST types
   - Methods operating on AST

2. **Wrong lifetime annotations** (~50 errors)
   - Functions with multiple lifetimes  
   - Structs with borrowed data

3. **Allocation sites** (~46 errors from parser modules)
   - Still using `Box::new()` instead of arena

### 🎯 Next Steps (Priority Order)

1. **Update all Parser methods** to use `alloc_expr/alloc_stmt`
   - Start with `expression_parser.rs`
   - Then `statement_parser.rs`
   - Then `item_parser.rs`

2. **Update CodeGenerator** with `'ast` lifetime
   - Add lifetime to struct
   - Update all methods
   - Fix all `Box::new()` → arena references

3. **Update Analyzer** methods (already started)
   - All analysis methods
   - All optimization passes

4. **Update Salsa database** (compiler_database.rs)
   - This is tricky - Salsa has its own lifetime management
   - May need to clone AST into database

5. **Update optimizer passes** (15 files)

6. **Update all tests**

7. **Reduce stack size** 64MB → 8MB and verify

### 🔧 Technical Approach

**Lifetime Management:**
- Parser owns arenas with 'static storage lifetime
- `parse()` returns `Program<'parser>` where 'parser is Parser's borrow lifetime
- Using `unsafe` transmute to bridge arena 'static → Parser lifetime
- Safe because Parser owns arenas; references can't outlive Parser

**Arena Usage:**
```rust
// Parser helper methods
pub fn alloc_expr<'parser>(&'parser self, expr: Expression<'static>) 
    -> &'parser Expression<'parser> 
{
    unsafe { std::mem::transmute(self.expr_arena.alloc(expr)) }
}
```

**Why Safe:**
- Parser owns the arena
- Arena lives as long as Parser
- References allocated from arena can't outlive Parser
- Transmute just changes the type-level lifetime annotation

### 📈 Estimated Remaining Time

- Parser modules: 4-6 hours
- CodeGenerator: 3-4 hours
- Analyzer completion: 2-3 hours
- Optimizer passes: 2-3 hours
- Compiler database: 1-2 hours
- Tests: 2-3 hours
- Verification: 1 hour

**Total:** ~15-22 hours remaining

### 💡 Lessons Learned

1. **Lifetime elision doesn't work** with complex nested structures
2. **Every function** that touches AST needs explicit lifetime
3. **Vec<Expression>** needs `Vec<Expression<'ast>>`
4. **HashMap values** need lifetime if they contain AST
5. **Salsa integration** may require cloning AST (upcoming challenge)

### 🚀 Benefits (Once Complete)

- ✅ **No stack overflow** - eliminates recursive Drop
- ✅ **Normal stack size** - can use 8MB instead of 64MB
- ✅ **Better performance** - contiguous memory, faster allocation
- ✅ **Less memory** - no per-Box overhead
- ✅ **Proper solution** - no workarounds, no tech debt

---

**Branch:** `feature/fix-constructor-ownership`  
**Last Commit:** `64f218c2`  
**Status:** Does not compile (396 errors - expected)  

**To Continue:**
```bash
git checkout feature/fix-constructor-ownership
cargo check 2>&1 | less  # See all errors
```

Start with fixing parser modules, then codegen, then optimizer.

