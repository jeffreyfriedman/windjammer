# Windjammer quick sheet

Windjammer (.wj) is a Rust-family language with inferred ownership. You have not seen it in training: do not guess from Rust or Python.

- Types: `int` (64-bit), `float`, `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` (`use std::collections::HashMap`).
- No semicolons. `let mut` to mutate. The last expression is the block value. `if`/`match` are expressions. `[1, 2]` is a Vec (no `vec!`).
- Strings interpolate: `println("${x} ${y:.2}")`; `println` is a function, not a macro. Use `string`, never `String`; literals are already `string` (no `.to_string()`).
- Ownership is inferred: write `fn f(v: Vec<int>)`, `self`; never `&`, `&mut`, `.iter()`, `.unwrap()`, `.clone()` unless needed. Iterate with `for x in v`; adapters start at the collection: `v.map(..)`.
- Read all input: `use std::fs`, then `let text = fs::read_to_string("/dev/stdin").unwrap_or("")` and `for line in text.lines()`. `std::env::args()` is an iterator.
- `len()` is `usize` when compiled: write `let n: int = v.len() as int`. Slice strings with `s.as_str()[a..b]`.
- Printing `Option`/`Result`/`Vec` needs `${x:?}`. HashMap order is unspecified: sort keys before printing.
- Build with `wj build main.wj`; run with `wj run --compiled main.wj` (input on stdin).

Tools: when a compiler message or a library call is unclear, run `wj docs <terms>` (search the language guide and stdlib) and `wj explain <CODE>` (explain an error code) before guessing.
