# Fuzzing Windjammer

This directory contains fuzz targets for testing the robustness of the Windjammer compiler.

## Prerequisites

Install cargo-fuzz:

```bash
cargo install cargo-fuzz
```

## Running Fuzz Tests

### Fuzz the Lexer

```bash
cargo fuzz run fuzz_lexer
```

This tests that the lexer never panics on any input, including:
- Invalid UTF-8
- Malformed tokens
- Edge cases in string/number parsing

### Fuzz the Parser

```bash
cargo fuzz run fuzz_parser
```

This tests that the parser never panics on any token stream, including:
- Invalid syntax
- Unexpected token sequences
- Deeply nested structures

### Fuzz the Code Generator

```bash
cargo fuzz run fuzz_codegen
```

This tests that codegen never panics on any valid AST, including:
- Complex type structures
- Edge cases in optimization
- Unusual control flow

## Continuous Fuzzing

For continuous integration, run fuzzing for a fixed duration:

```bash
# Run each fuzzer for 60 seconds
cargo fuzz run fuzz_lexer -- -max_total_time=60
cargo fuzz run fuzz_parser -- -max_total_time=60
cargo fuzz run fuzz_codegen -- -max_total_time=60
```

## Corpus

The fuzzer automatically builds a corpus of interesting inputs in `fuzz/corpus/`.
These inputs can be used as regression tests.

## Artifacts

When a crash is found, it's saved in `fuzz/artifacts/` for debugging.

## Coverage

To run with coverage:

```bash
cargo fuzz coverage fuzz_lexer
cargo fuzz coverage fuzz_parser
cargo fuzz coverage fuzz_codegen
```

## Integration with CI

Add to `.github/workflows/fuzz.yml`:

```yaml
name: Fuzzing

on:
  schedule:
    - cron: '0 0 * * *'  # Daily
  workflow_dispatch:

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run fuzz_lexer -- -max_total_time=300
      - run: cargo fuzz run fuzz_parser -- -max_total_time=300
      - run: cargo fuzz run fuzz_codegen -- -max_total_time=300
```

