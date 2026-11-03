# Windjammer for agents

Windjammer (.wj) is a Rust-family language with inferred ownership. It is not in your training data: do not guess from Rust or Python.

- Types: `int` (64-bit), `float`, `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` (`use std::collections::HashMap`).
- No semicolons. `let mut` to mutate. The last expression is the block value. `[1, 2]` is a Vec (no `vec!`).
- Input: `use std::fs`, `let text = fs::read_to_string("/dev/stdin").unwrap_or("")`, `for line in text.lines()`. `std::env::args()` is an iterator.
- `len()` is `usize` when compiled: `let n: int = v.len() as int`. Slice strings with `s[a..b]`.
