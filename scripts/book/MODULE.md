# scripts/book

## Purpose
The sample runner of The Windjammer Book (`docs/book`), in interpreted Windjammer. It runs every program extracted from the book's pages.

## Inputs / outputs
- In: a samples directory written by `wj-docs book-extract DIR` (`ID.wj`, `ID_test.wj`, optional `ID.expect`, `INDEX.tsv` mapping each id to `page.md:line`), the `wj` binary to test, a scratch directory.
- Out: one `FAIL page.md:line ...` entry per failing sample and a summary line; exit status 1 when any sample fails, 2 on bad usage.

## Layout
- `book_samples.wj`: the runner. Interpreted pass (exit status 0, stdout equal to `ID.expect` when present; `_test` files through `wj test --interpret`), then with `--compiled` one shared project with a `cmd/ID/main.wj` per sample, run with `wj run --compiled --bin ID`, whose stdout must equal the interpreted stdout.
- `wj.toml`: the capability grant (scratch space and the programs it starts).

## Invariants
- The extraction (hidden `~` lines, `// expect:` lines, which fences count) is `tools/docs_truth/book.rs`; the runner knows nothing about Markdown.
- `tests/book_test.rs` runs it with the freshly built `wj`, so a language change that breaks a sample fails the compiler's own test run; its mutation tests plant a wrong expectation and a type error and require the runner to fail.
