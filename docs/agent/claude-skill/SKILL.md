---
name: windjammer
description: Write, check, run, test and build Windjammer (.wj) code. Use when a task touches .wj files, a wj.toml project, or the `wj` CLI; Windjammer is absent from model training data, so follow this skill instead of guessing from Rust or Python.
---

# Windjammer skill

Windjammer is a Rust-family language with inferred ownership that transpiles to Rust. Do not write Rust syntax.
Read `docs/agent/WINDJAMMER_FOR_AGENTS.md` (or `wj docs idioms`) once per session; it is the context pack.

## The loop (always in this order)

1. Write or edit the `.wj` file (small steps; one function at a time).
2. `wj check --json FILE_OR_DIR`: front-end check without cargo or rustc. Output is one JSON object
   (docs/diagnostics-schema.json) `{success, diagnostics:[{code, severity, message,
   primary_span:{file,line_start,col_start,byte_start,byte_end}, help, fixes}]}`.
   Exit 1 when `success` is false. Fix every error; `wj check --fix` applies `machine_applicable` fixes;
   `wj explain CODE` explains any code.
3. `wj run --interpret FILE`: instant execution (no cargo, no rustc). stdout is program output only.
   Exit 101 is a panic, 2 means the construct is unsupported in the interpreter.
4. `wj test --interpret DIR_OR_FILE`: runs `@test`, `test_*` and `@test_cases` functions in `*_test.wj`.
5. `wj build FILE_OR_DIR`: the production path (Rust + cargo). Run it last; if it fails on code the interpreter
   runs, shrink to a repro and report it rather than contorting the source.

## Look things up, do not guess

| Need | Command |
|------|---------|
| a method or stdlib function | `wj docs <terms>` (offline; e.g. `wj docs hashmap entry`, `wj docs sort_by_key`) |
| an error code | `wj explain WJ0001` |
| a worked example | `wj docs <tag>` lists `examples/verified/*.wj`; each file is a complete program with its output |
| Rust-ism migration | `wj lint FILE`, `wj fix PATH` |
| formatting | `wj fmt FILE` |

## Rules of thumb

- `println("${x}")`, never `println!`; `[1, 2]`, never `vec![]`; no `#[derive]`, no `&`/`&mut`, no `.iter()`,
  no `.unwrap()`/`.expect()`, no `"lit".to_string()`, no trailing `return`, no semicolons.
- Mutation needs `let mut`; method receivers are plain `self` and mutation is inferred.
- Use `int`/`float`/`string`; handle `Result`/`Option` with `match`, `?`, `unwrap_or`.
- Print `Result`/`Option`/`Vec` with `${x:?}`. Sort `HashMap` keys before printing.
- Never claim code works from `wj check` alone: show the `wj run --interpret` or `wj test --interpret` output.

## MCP

If the `windjammer-mcp` server is configured, prefer its tools: `check_code`, `run_code`, `test_code`,
`format_code`, `explain_error`, `docs_search` (same behaviour as the CLI commands above).
