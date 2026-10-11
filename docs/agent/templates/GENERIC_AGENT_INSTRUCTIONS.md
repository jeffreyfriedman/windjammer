# Windjammer instructions for any coding agent

Paste into your tool's rules file (Cursor rules, Copilot instructions, Windsurf rules, Aider conventions,
Codex/Gemini/other `AGENTS.md`-style files).

Windjammer (`.wj`) is a Rust-family language with inferred ownership. It is absent from model training data,
so do not guess syntax from Rust, Go, Python or TypeScript; look it up with `wj docs <terms>` (offline) and read
`docs/agent/WINDJAMMER_FOR_AGENTS.md` once.

Verification loop for every change, in order:

1. `wj check --json <path>`  (parse + lint + transpile, JSON diagnostics with spans; exit 1 on errors)
2. `wj run --interpret <file>`  (instant run, stdout is program output)
3. `wj test --interpret <dir>`  (`@test` / `test_*` functions in `*_test.wj`)
4. `wj build <path>`  (production build)

Never report success without showing step 2 or 3 output. Surface syntax: `println("${x}")`, `[1, 2]`, no macros,
no `#[derive]`, no `&`/`&mut`, no `.iter()`, no `.unwrap()`, `let mut` for mutation, plain `self` receivers,
`int`/`float`/`string`, `Result`/`Option` with `match`/`?`/`unwrap_or`.
