# Book roadmap

This note tracks where the book is complete and where the next chapters go. It is not part of the published book; the
book's gates (`tests/book_test.rs`, `tests/docs_truth_test.rs`, the feature registry in `features.toml`) hold the
published pages to what the compiler does today.

## Coverage against a complete language book

| Area | Where it lives | Status |
|---|---|---|
| Getting started: install, first project | getting-started/ | complete |
| Common concepts: values, numbers, strings, collections | tutorial/values-and-types, numbers, collections-and-strings | complete |
| Control flow, patterns and match | tutorial/control-flow | complete |
| Ownership, the Windjammer way | tutorial/ownership, reference/ownership-rules, showcase/ownership | complete |
| Structs, enums, traits, generics | tutorial/types-and-traits | complete |
| Errors | tutorial/errors, runtime-errors, reference/error-model | complete |
| Closures and iterators | tutorial/closures, iterators | complete |
| Modules and packages | tutorial/modules, reference/modules-and-visibility | complete |
| Testing | tutorial/testing | complete |
| Concurrency and async | tutorial/concurrency, async | complete |
| Capabilities and effects | tutorial/effects, reference/effects-model, showcase/authority | complete |
| Networking: HTTP and raw sockets | reference/std, tutorial/sockets | sockets chapter new; an HTTP client and server chapter is next |
| Rust interop | tutorial/rust-interop, showcase/rust-crates | complete |
| Targets: Rust, Go, JavaScript, TypeScript, WebAssembly, WGSL | tutorial/targets, shaders, reference/targets | complete |
| Tooling: fmt, check, fix, run, replay, cache | tutorial/tooling, repl, time-travel | complete; the build memory report is documented once it reaches the integration branch |
| Playground | tutorial/playground | complete; a multi-file editor on the page is the next step |
| Appendices | appendix/ | migration guides for Rust, Go and TypeScript |

## Next chapters, highest value first

1. **Implementing the iterator trait.** The iterators chapter covers the adapters on lists and ranges. A chapter on writing
   your own iterator type belongs after the trait surface for it is registered in the feature registry.
2. **HTTP clients and servers** (`std::net::http`), with record and replay for tests, in the style of the sockets chapter.
3. **Smart pointers and shared state**: `Rc`/`Arc`, `Mutex` and interior mutability from the Windjammer side.
4. **A second final project** that uses packages, a Rust crate and a capability grant together.
5. **The agent loop end to end**: write, `wj check --error-format agent`, `wj fix`, `wj run --verify`, `wj replay --json`,
   as one worked session whose transcript is verified.
6. **Playground**: Run links that open multi-file projects; inline output under each fence.

## Compiler gaps the samples revealed

These are reported as roadmap items for the compiler; the book's samples are written in the forms that run everywhere
today.

- `v.filter(..).map(..).collect()` compiled needs a `.cloned()` after `filter` and a separate step; the interpreter
  accepts the one-line chain.
- `Vec::rev()` / `.skip().take()` on a typed list compile with an integer-width mismatch (`expected uint, found int`);
  ranges compile.
- A struct literal in the scrutinee of `if let` (`if let P { .. } = P { .. } {`) is rejected by the generated code
  (WJ0099); binding the value first works.
- The Option-versus-Result pattern diagnostic prints "an `Result` pattern" (article); wording only.
