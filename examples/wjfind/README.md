# wjfind

A file-search command-line program (in the spirit of `ripgrep`) written in Windjammer: argument parsing, directory walking,
`.gitignore` handling, pattern matching, output formatting and replace. It is a multi-file project; the entry point is `src/main.wj`.

Status: `tests/examples_gate_test.rs` parses and analyzes every file here (`examples/EXAMPLES.toml`). It is not built or benchmarked in CI,
and this README makes no performance claims; run `wj build` in this directory to compile it.
