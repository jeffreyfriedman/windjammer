# scripts/book

## Purpose
The sample runner of The Windjammer Book (`docs/book`), in interpreted Windjammer. It runs every program extracted from the book's pages.

## Inputs / outputs
- In: a samples directory written by `wj-docs book-extract DIR` (`ID.wj`, `ID_test.wj`, optional `ID.expect`, `INDEX.tsv` mapping each id to `page.md:line`), the `wj` binary to test, a scratch directory.
- Out: one `FAIL page.md:line ...` entry per failing sample and a summary line; exit status 1 when any sample fails, 2 on bad usage.

## Layout
- `book_samples.wj`: the runner. Interpreted pass (exit status 0, stdout equal to `ID.expect` when present; `_test` files through `wj test --interpret`), then with `--compiled` one project per sample (bins of one project share function signatures by name), run with `wj run --compiled`, whose stdout must equal the interpreted stdout. Multi-file projects under `projects/` run from their `.root`, interpreted and compiled, against their `.expect` lines.
- `wj.toml`: the capability grant (scratch space and the programs it starts).
- `--wasm RUNNER` / `--ts-playground RUNNER` (ADR-0034): every Windjammer sample also runs through the browser playground's wasm interpreter (`docs/book/playground/run-node.mjs`) and must equal native `wj run --interpret --sandbox --simulate --seed 0 --limits ...`; every `ts,run` companion also runs through the playground's TypeScript path (`run-ts-node.mjs`) and must equal `node --experimental-strip-types`.
- `playground_check.wj`: builds the wasm interpreter (`crates/windjammer-playground`), checks the generated `ts-sandbox.html`, extracts the samples and runs the two playground passes.

## Invariants
- The extraction (hidden `~` lines, `// expect:` lines, which fences count) is `tools/docs_truth/book.rs`; the runner knows nothing about Markdown.
- `tests/book_test.rs` runs it with the freshly built `wj`, so a language change that breaks a sample fails the compiler's own test run; its mutation tests plant a wrong expectation and a type error and require the runner to fail.
- The runner has one test hook, `BOOK_PLANT_COMPILED_DIFF=SUBSTR` (appends a line to the compiled output of matching samples); the compiled-gate mutation test uses it, so it needs no real compiler gap, and `mutation_tests_do_not_depend_on_known_gaps` enforces that.
