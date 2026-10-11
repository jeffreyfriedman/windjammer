---
status: accepted
last_verified: 203e43d114c9
---
# Parser arena ownership

## Problem

`Parser::parse()` returns `Program<'static>`. Its expression, statement and pattern nodes are allocated in
`typed_arena::Arena`s that the `Parser` owns, and the `'static` lifetime is a transmute (`alloc_expr`,
`alloc_stmt`, `alloc_pattern` in `src/parser_impl.rs`). Nothing in the type system ties a `Program` to the
parser that allocated it, so the idiom `let program = Parser::new(tokens).parse()?;` (the parser is a temporary)
freed the arenas under a live program: a use after free. It showed up as segfaults in the doc-fence tests until
the parser was leaked by hand at the call site.

## Fix (shipped): programs own their arenas

```text
pub struct Program<'ast> { pub items: Vec<Item<'ast>>, owner: AstOwner }   // src/parser/ast/core.rs
pub struct AstOwner(Vec<Arc<AstArenas>>);                                   // src/parser/ast/owner.rs
pub struct AstArenas { expr, stmt, pattern: typed_arena::Arena<..>, ledger }
```

* The parser allocates into an `Arc<AstArenas>` bundle. `parse()` *seals* it: the `Arc` moves into the returned
  program's `AstOwner` and the parser starts a fresh bundle, so a parser never allocates into a bundle that a program
  can see. The parser can be dropped at any time; the arenas are freed when the last program clone is dropped.
* `AstArenas` is `Send + Sync` by the sealing argument (no `alloc` after sealing; documented at the `unsafe impl`).
  `Parser` is `!Sync` (it allocates through `&self`). The string-interpolation sub-parser shares its parent's bundle
  (`Parser::new_sharing_arenas`) instead of being leaked.
* Building a program from another program's items carries the owner: `Program::from_items_of(items, sources)`
  (module merging in `file_compiler.rs`, the JS tree shaker). `Program::new(items)` has no owner and is for empty or
  hand-built programs (and temporary views whose source program outlives them). `Program { items }` without
  an owner no longer compiles (the field is crate-private and every literal must name it), so a forgotten owner is visible.
* Long-lived holders keep owners, not parsers: `ModuleCompiler::_arena_owners` (shared analyzer, trait registry).
  Salsa (`compiler_database.rs`, LSP `database.rs`) stores the program; evicting a revision frees its AST.
* The `'static` in `Program<'static>` remains a promise, not a proof: an `Item` *cloned out* of a program and kept
  after every owner is dropped would dangle. Known holders keep an owner; the interpreter still `Box::leak`s its
  programs (process-lifetime, one per loaded file; removing those is the remaining step and needs the interpreter's
  `&'static` node references replaced by owner-carrying handles).

Gates: `tests/parser_arena_soundness_test.rs` (drop order, clone/merge co-ownership, cross-thread drop, fresh bundle
per parse) and `tests/parser_arena_memory_test.rs` (counting allocator: 1000 parses of a small file grow live heap
by < 1 MB; a mutation that leaks the arenas again must exceed the gate).

Kill criterion (kept for reference): if a future refactor needs more than a mechanical owner hand-off, fall back to
an LSP parse cache keyed by file version instead of leaking.
