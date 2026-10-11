# Lexer panics on integer literals above i64::MAX

Minimal repro (`repro.wj`):

```
fn main() {
    let h: u64 = 14695981039346656037
    println!("{}", h)
}
```

`wj build repro.wj` panics at `src/lexer/number_lexing.rs:210` with
`ParseIntError { kind: PosOverflow }` instead of lexing a `u64` literal.
Found while writing the `benchmarks/rust_parity` kernels (FNV offset basis);
the kernels use a smaller constant to avoid it.
