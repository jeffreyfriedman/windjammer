---
status: proposed
---
# ADR-0016: Arenas as a user capability

- **Status:** Proposed
- **Date:** 2026-10-11
- **Related:** ADR-0001 (ownership truth), ADR-0002 (local mutability), ADR-0006 (optimizer), ADR-0008 (stdlib portability),
  ADR-0014/0015 (surface), `crates/windjammer-runtime/src/arena.rs`, `src/analyzer/arena_escape.rs`,
  `src/interpreter/builtins/arena.rs`, `src/optimizer/passes/scratch_reuse.rs`, `benchmarks/arenas/`,
  [parser arena ownership](../design/parser-arena-ownership.md) (the compiler's own arenas; separate concern)

## Context
High-performance programs (game loops, request handlers, compilers) allocate many short-lived objects that die
together: everything spawned in a frame, everything parsed for one request, every scratch string of one phase.
Per-object allocation pays a `malloc`/`free` pair per object and scatters the objects in memory. Arenas allocate by
bumping a pointer (or pushing into one contiguous buffer) and free everything with one reset.

Rust offers this through crates (`bumpalo`, `typed-arena`, slotmaps). Their cost is lifetime plumbing: a bump
reference is `&'bump T`, which infects every struct and signature that holds one, and mistakes surface as borrow
checker errors about generated lifetimes. Windjammer infers ownership and has no lifetime annotations (ADR-0001),
so it can offer the same performance with the lifetime reasoning done by the compiler.

## Decision summary (recommended; implemented behind the `unstable_arenas` compiler feature)
1. **Library types, no new syntax (phase 1):** `use std::arena` gives `Arena<T>`, `Handle<T>` and `Scratch`.
2. **Typed arenas hand out `Copy` handles,** not references: `let h = particles.alloc(p)`, `particles.get(h)`,
   `particles.get_mut(h).x += 1`, `particles.reset()`. A handle is two `u32`s plus an arena id; debug builds check
   the generation and the arena id on every access.
3. **`Scratch`** is an untyped bump allocator for transient strings and lists: `s.str("..")`, `s.new_string()`,
   `s.new_list()`, `s.reset()`. In Rust it is `bumpalo`; the borrow checker ties every scratch value to the scratch.
4. **Escape check (WJ0160):** a value allocated in an arena declared in a function may not leave the arena's scope
   (returned, assigned to an outer variable or field, stored into an outer collection, used as a block value, or
   used after `reset()`). Inferred, no annotations; error with a fix hint. Runs on the compiler and interpreter paths.
5. **Auto-arena (opt-in optimizer pass `scratch_reuse`):** per-iteration `String::new()`/`Vec::new()` buffers that
   are only borrowed are hoisted out of the loop and `clear()`ed, behind `OptimizationConfig::with_auto_arena()`.
6. **Backends:** Rust uses the runtime types; Go/JS/WASM fall back to their GC heap with identical semantics
   (an `Arena<T>` is a growable array plus a generation counter; `Scratch` values are plain strings/lists);
   the interpreter implements the debug-build semantics, including handle checks.

## Options

### A. Surface: library types (recommended) vs new syntax
- **A1 library types + inferred escape check (chosen).** `let frame = Scratch::new()` inside a block *is* a scoped
  arena: its scope is the block, and the escape check proves nothing allocated in it leaves the block. No keyword,
  no grammar change, no AST variant, so the Go/JS/interpreter lowerings need no new node type.
- **A2 `scope arena frame { .. }` block.** Sugar for `{ let frame = Scratch::new(); .. }`. Reads well and makes the
  intent obvious to agents, but costs a keyword and parser/AST/formatter/LSP work in every backend for no new
  capability. *Owner decision:* add it in phase 2 if usage shows people want it (the desugaring is fixed: block +
  `let`, so it can be added without changing semantics).
- **A3 `arena()` free function / `frame.alloc(x)` returning a reference-like `T`.** Closest to the sketch in the
  request, but a reference-returning `alloc` needs either lifetimes in emitted Rust structs (the bumpalo problem)
  or hidden indirection. Handles are explicit, `Copy`, serialisable and safe in every backend.

### B. Runtime: own type vs bumpalo
- **B1 own `Arena<T>` (Vec-backed, chosen for typed arenas).** Contiguous storage, `iter_mut` over all live
  values (what a particle system wants), reset keeps capacity: steady-state frames allocate nothing. Handles
  are indices, so no lifetimes appear in user types and the same design works on GC backends.
- **B2 `bumpalo` (chosen for `Scratch`).** Mature, fast, supports strings and vectors in the arena. Its values
  borrow the arena, which is fine for scratch data that never leaves a function (the escape check guarantees it).
- Rejected: emitting `bumpalo` references for typed values (lifetime plumbing in generated structs), and a
  generational slotmap with free lists (per-object free is not the arena use case; can be added as `Pool<T>`).

### C. Escape check placement
- **C1 analyzer fact pass over the AST, before codegen (chosen for now).** Pure, cheap, runs in the compiler,
  the IDE analysis and the interpreter loader through one entry (`arena_escape::gate`).
- **C2 IR solver constraint.** ADR-0001 makes the IR solver the owner of ownership decisions; the escape check is
  a diagnostic, not an ownership decision, but region inference proper (nested arenas, arenas passed through
  calls) belongs there. *Owner decision:* move the check into `src/ir` once the IR carries region facts.

### D. Feature gating
`use std::arena` without the compiler feature is error WJ0161. Gate removal criterion: conformance programs pass
on Rust and interpreter, Go/JS lowerings exist, and the benchmark keeps its win (see kill criteria).

## Semantics
- `Arena<T>`: `new`, `with_capacity`, `alloc(v) -> Handle<T>`, `get(h)`, `get_mut(h)`, `try_get(h) -> Option`,
  `iter`, `iter_mut`, `len`, `is_empty`, `reset`, `generation`. `reset` drops every value (destructors run) and
  bumps the generation. Release builds skip the generation check: a stale handle then reads another live value or
  panics on bounds, a logic error but never memory unsafety. `try_get` always checks.
- `Scratch`: `str(s)`, `new_string()`, `new_list()` (`ScratchString`/`ScratchList`: `len() -> int`, `push`/`push_str`, slices), `alloc(v)` (Copy values only), `reset`, `allocated_bytes`.
- Escape rules (WJ0160), per function, no annotations: a value is *arena-bound* if it comes from an allocation
  method of a local arena, is a copy of an arena-bound variable, or is a struct/tuple/list literal or block value
  containing one. Copies out (`to_string`, `to_vec`, `push_str`) are not arena-bound. Arenas reached through a
  parameter or `self` are external: their values may leave the function, but use after `reset()` is still an error.
  The check is conservative for a `reset()` inside a branch (a later use is reported).

## Auto-arena (`scratch_reuse`) safety sketch
Rewrite `for .. { let mut b = String::new(); <uses> }` to `{ let mut b = String::new(); for .. { b.clear(); <uses> } }`.
Sound when (1) `b` is not mentioned before its `let` or in the loop header and has no other binding in the loop, so
every use refers to the same binding before and after; (2) every use borrows `b` (whitelisted non-consuming methods,
`&b`, `&mut b`, `b[i]`, `b += ..`, format-family macro arguments), so no iteration can observe a value that was moved
or that survives into the next iteration other than through `b` itself, and `clear()` makes `b` equal to a fresh
`new()` (same contents, length 0; only `capacity()` differs, which no borrowed use can tell apart from a fresh buffer
except `capacity()` itself); (3) for `Vec`, element destructors run at the next iteration's `clear()` instead of at
the end of the iteration, so the pass refuses files with `impl Drop` and loop headers that lock or borrow. The
wrapping block frees the buffer when the loop exits. Tests: `tests/optimizer_scratch_reuse_test.rs` (rewrites,
nine refusal shapes, a compiled fixture against an independent Python oracle, and a mutant `ScratchIgnoresMoves`
that the fixture catches). Promotion of non-buffer allocations (many small `Box`/struct allocations freed together)
into an `Arena` needs type facts the text-level optimizer does not have; it is listed as phase 2 for the IR.

## Results
See `benchmarks/arenas/RESULTS.md` (numbers, machine load at measurement time, and where arenas lose).

## Kill criteria
- If `Arena<T>` is not at least 1.5x faster than per-object `Box` allocation on the particle and tree workloads
  (release, quiet machine), or `Scratch` is not faster than `String::new()` per label, drop the runtime types and
  document `Vec` + index patterns instead.
- If the escape check produces a false positive on any conformance or dogfood program that cannot be fixed by a
  rule change within the same release, demote WJ0160 to a warning.
- If `scratch_reuse` shows no measurable win on the scratch benchmark, or any miscompile is found, delete the pass.

## Decision needed from: project owner
1. Accept A1 (library types, no syntax) now and defer A2 (`scope arena` block)? 2. Accept handles over references
(A3 rejected)? 3. Move the escape check to the IR (C2) as a follow-up? 4. Turn `scratch_reuse` on by default after a
release of soak?
