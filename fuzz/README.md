# Fuzz targets

[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets in `fuzz_targets/`: `fuzz_lexer`, `fuzz_parser`, `fuzz_codegen`.
They feed arbitrary bytes to the compiler front end and fail on any panic.

```bash
cargo install cargo-fuzz
cargo fuzz run fuzz_lexer -- -max_total_time=60     # from the repository root, nightly toolchain
```

Crashes are written to `fuzz/artifacts/`; promote each one to a regression test.
