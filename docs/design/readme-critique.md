---
status: current
last_verified: 203e43d114c9
---
# README critique (before the rewrite)

Basis: the README at commit 203e43d1 (207 lines), compared from memory of the READMEs of Rust, Go, Zig, Gleam, Mojo,
Bun, Deno, Astro, Svelte and Tauri. No web fetch was needed for the patterns below; they are stable and well known.

## What a skimmer saw in 10 seconds
- Title, a one-line definition ("compiles to Rust, ownership inferred"), two license badges.
- A paragraph of disclaimers ("describes only what exists", "honest ranked list of gaps").
- A version line, then a long bullet list of syntax. No code until line 100.
- Result: a reader learns the project is cautious, not why they should care. An agent finds no pointer to the agent
  guide, `llms.txt` or MCP, although all exist.

## What worked
- Honesty is a real differentiator and is backed by machinery (generated regions, verified fences). Keep it.
- The generated CLI table cannot drift.

## What was confusing or missing
- No positioning beyond a definition; no "why this and not Rust/Go" (Gleam, Zig, Mojo lead with it).
- No side-by-side sample. Every strong README shows the pain removed (Svelte: boilerplate; Bun: a one-liner speed
  claim with a chart; Deno: `deno run` of a URL).
- Quickstart begins with `cargo build` of the compiler and a legacy-binary aside; the first runnable program is
  at line 100. Go/Zig/Gleam get to a running program within a screen.
- Targets table hid the numbers that exist (conformance matrix) behind "proposed tiers".
- The interpreter (`wj run --interpret`), the most attractive property for trying the language, is a footnote.
- Agent features (JSON diagnostics, MCP, `wj docs`, the agent-dx benchmark) are absent from the README.
- Internal contributor material (test commands, hardening rules, cache setup, ADR numbers) sits on the front page
  and belongs in CONTRIBUTING.md and AGENTS.md.
- Badges: license only. No CI or version badge.

## Untrue or unverifiable at review time
- `Cargo.toml` description "80% of Rust's power with 20% of the complexity" is an unmeasured slogan; the VS Code
  extension carries the same line (and version 0.35.0 against 0.50.0).
- README caveat "`wj check` runs `cargo check`" contradicts the agent guide's `wj check --json` ("parse + lint +
  transpile, no cargo"); the two must be reconciled by the owner of the check work (see the rewrite's note).
- A hand-typed test count and run time in the Test section: a statistic that goes stale (it is gone from the rewrite).
- Performance: `benchmarks/rust_parity/results.json` exists but is not CI-written (`measured.json` is empty), so the
  rewrite quotes no speed numbers.

## Rewrite principles
1. One sentence of positioning, three reasons to care, each one backed by a command the reader can run.
2. Code before prose. Quickstart in `sh,verify` fences; feature tour in `// expect:` fences (interpreted in CI).
3. Side-by-side sample is a test (`tests/readme_samples_test.rs`): both programs compile and print the same output,
   and the README must contain both files verbatim.
4. Status is generated: the backend table is a `wj:gen` region; versions and counts come from metrics.
5. A "For AI agents" block near the top, pointing at files that exist.
6. Anything not measured is a goal, labelled as one (see docs/WHY.md).
