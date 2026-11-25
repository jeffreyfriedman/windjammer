---
status: current
last_verified: 9d529f6b21e1
---
# Soundness: parsed files shared between the tests of one `wj test --interpret` run

Finding: `run_one_test` (`src/interpreter/runner.rs`) builds a fresh `Interpreter` per test, and `load_test_program`
(`src/interpreter/loader.rs`) parses the whole project again for each: every source file, the test file, and the files
whose private struct names are rewritten (`parse_files`, `src/interpreter/project.rs`, parses those twice). The text
of the files is the same for every test of a run, so a package of N tests parsed each file about N times, and leaked
each parse (`Box::leak`) for the life of the process.

What a parse reads. `parse_source_renaming` is the lexer, the identifier renames, the parser and two gates
(`arena_escape::gate`, `task_structure::gate`) applied to the result. Its inputs are the file's name, its text and the
renames; the lexer, the parser and the gates hold no static, no thread-local and no counter (nothing in `src/parser`,
`src/lexer` or the two gate modules is a `static`/`thread_local`/atomic except the arena-free ledger, which counts frees
and is not read by the parse). The compiler build and the `WJ_*` environment are constants of one process. So a parse is
a function of (name, text, renames); a failing parse is never stored (a refusal is recomputed, so a hit can only
return a parse that passed the gates once).

What is shared. The `&'static Program<'static>` of a file, keyed by the content key `note_parsed` already computes
(`content_key` in `src/interpreter/compiled_facts.rs`: name, text, renames, plus a format tag). A hit also calls
`note_parsed`, so the width share and the disk fact store still find the file's key under its name.

Does anything mutate an AST after the parse? The AST types (`src/parser/ast/`) hold no `Cell`, `RefCell`, `OnceCell`,
`Mutex`, atomic or raw pointer, and the interpreter holds the program only as `&'static` (shared references; no `*mut`
cast reaches a node: the only `*mut` in `src/interpreter` is the scheduler's `Interpreter` pointer). Desugaring and name
resolution annotations are produced by the parser before the program is returned; the resolver
(`resolve`) writes a side table, not the nodes. So a shared program is immutable data: two interpreters reading it
cannot observe each other. `Program` owns its arenas through `AstOwner` (`Arc`s); the arena type is `!Sync` only for
`alloc(&self)`, and sealed arenas are never allocated in again (`src/parser/ast/owner.rs`), which is why `AstArenas`
is `Send + Sync`.

Identity-keyed tables. The interpreter keys several caches by the address of an AST node (`loop_mut_cache` and
`MutScc` in `analysis.rs`, `scalar_progs` in `scalar_pipe.rs`, `compiled_closures` and `closure_cache` in
`resolved/closure.rs`, `body_use_modules` in `loader.rs`, `lit_widths_by_program` in `compiled_facts.rs`, the debugger's
statement ids, the OSR tables in `resolved/osr.rs`, `PlaceCode` addresses). Each is a field of the `Interpreter` (or of
a value the interpreter owns), created empty with it. No process-global or thread-local table in `src/interpreter` is
keyed by an address (the statics are `FACT_STORE` and `PARSED`, keyed by content; `OBSERVED` is a constant list). The
`Rc`-pointer keys (`Rc<FuncDef>`) name per-interpreter objects the register pass rebuilds for each interpreter, and the
per-function caches in them (`mutated`, `self_mutated`, `jit`) live in `FuncDef`, not in the AST. Before this change a
node address was unique per test because every parse was leaked and never freed; with the share two interpreters see the
same address for the same node, which is harmless exactly because every table that uses the address belongs to one
interpreter. The one object that outlives a test is the crate host (`CrateHost`, shared across tests since ADR-0031);
it is keyed by crate and symbol names, not by nodes.

Threads. The runner executes tests one after another on one big-stack thread (`with_big_stack`). The share lives in a
`Mutex` and is `Send + Sync` (the programs are `&'static` over `Send + Sync` arenas), so a future parallel runner stays
sound: a hit hands out the same read-only reference.

Lifetime. The leaked program is created once per distinct content and lives until the process exits, as each per-test
leak already did; the share removes the repeats, so memory falls from one program per (test, file) to one per distinct
file content.

The key and a changed file. Equal key means equal name, text and renames, hence an identical parse. An edited file has a
different text and so a different key and a fresh parse. A verifying share (`ParseShare::verifying`) parses afresh on
every hit and compares the canonical dump of the two (`source_map::elided_debug`); the gate requires zero differences.
The share lives for one test run (created in `run_specs_with`, passed down as a field, never a static).

Residual assumption: a file's text is read once per test (the read is not shared); the key hashes what was read, so a
file edited while a run is in progress yields a fresh parse for the tests that read the new text.

Counters: `WJ_JIT_DEBUG` prints `[fast] parseshare hits H misses M`; a miss is one parse (lex, parse, gates).

Pinned by `tests/interpreter_parseshare_test.rs`: a shared parse equals a fresh one over the repo programs and
ecosystem packages; a changed test file re-parses only its own text; N tests of a module parse each file once; test
results are independent of test order and of closures defined by an earlier test; mutations of the key (text or renames
ignored, no keying) fail those tests.
