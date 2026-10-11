---
status: accepted
last_verified: 3406ef05bb35
---
# Parser arena ownership

## Problem

`Parser::parse()` returns `Program<'static>`. Its expression, statement and pattern nodes are allocated in
`typed_arena::Arena`s that the `Parser` owns, and the `'static` lifetime is a transmute (`alloc_expr`,
`alloc_stmt`, `alloc_pattern` in `src/parser_impl.rs`). Nothing in the type system ties a `Program` to the
parser that allocated it, so the idiom `let program = Parser::new(tokens).parse()?;` (the parser is a temporary)
freed the arenas under a live program: a use after free. It showed up as segfaults in the doc-fence tests until
the parser was leaked by hand at the call site.

## Current fix (shipped)

The arenas are `ArenaSlot`s that are never freed. Dropping a `Parser` retires its arenas (counted in
`parser_impl::retired_arenas()`), so every `Program<'static>` stays valid for the life of the process.
`tests/parser_arena_soundness_test.rs` drops the parser first, then walks the whole program, and asserts that
`parser_impl::freed_arenas()` stays 0; a mutation test shows the counter fires if an arena is ever freed.

Cost: AST memory of every parse lives until process exit. That is fine for `wj build/run/check` and for tests,
and a growth risk for long-lived processes (the LSP) that re-parse on every edit.

## Target design (not done: touches every call site and `src/parser`)

`Program` owns its storage:

```text
pub struct Program<'ast> { pub items: Vec<Item<'ast>>, owner: AstOwner }
struct AstOwner(Arc<AstArenas>);   // AstArenas { expr, stmt, pattern: Arena<..> }, Send + Sync by construction
```

* The parser allocates from `Arc<AstArenas>` and hands a clone to the `Program` it returns.
* `Program: Clone` clones the `Arc`; any API that moves `Item`s out of a program (about 17 struct-literal sites
  build `Program { items }`) must carry the owner (`Program::new(items, owner)`), which turns every "forgot the
  owner" bug into a compile error instead of a use after free.
* The lifetime parameter becomes real (`Program<'a>` borrowing from its own owner) or stays `'static` behind
  the `Arc`, which keeps `&'static Program<'static>` interpreter code unchanged; the interpreter's `Box::leak`s
  can then go.
* Kill criterion: if the 17 literal sites plus the roughly 530 `.items` uses need more than a mechanical
  `owner: program.owner.clone()` change, keep the leak and add an arena-recycling `Parser::reset()` for the LSP instead.

Prerequisite: land after `wt/parser-gaps` (it edits `src/parser`), so the refactor is one mechanical commit.
