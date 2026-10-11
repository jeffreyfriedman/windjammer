# Agent instructions for this Windjammer project

This project is written in Windjammer (`.wj`), a Rust-family language with inferred ownership. It is not in
your training data: do not write Rust or Python syntax. Context pack: `docs/agent/WINDJAMMER_FOR_AGENTS.md`
(copy it into the project, or run `wj docs idioms`).

## Workflow (every change)

1. Edit `.wj` files in small steps.
2. `wj check --json <path>`: fix every diagnostic (JSON with spans and repair hints; exit 1 on errors).
3. `wj run --interpret <file>`: run it and read the output.
4. `wj test --interpret .`: tests are `@test` / `test_*` functions in `*_test.wj` files.
5. `wj build`: production build. Report compiler bugs with a minimal repro instead of working around them.

Before writing an unfamiliar construct, run `wj docs <terms>`; it searches stdlib signatures, built-in methods,
idioms and 150+ verified example programs offline.

## Style rules

- `println("${x}")`, `[1, 2]`, no macros, no `#[derive]`, no `&`/`&mut`, no `.iter()`, no `.unwrap()`/`.expect()`.
- `let mut` for mutation; receivers are plain `self`; types are `int`, `float`, `string`, `Vec<T>`, `HashMap<K, V>`.
- Errors are `Result`/`Option` with `match`, `?` and `unwrap_or`.
- Format with `wj fmt`; check Rust-isms with `wj lint`.

## Project layout

```
wj.toml            project manifest
src/main.wj        entry point (fn main)
src/<module>.wj    one module per file, imported with `use <module>::name`
tests/*_test.wj    tests
```
