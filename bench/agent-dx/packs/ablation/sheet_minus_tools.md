# Windjammer for agents

Windjammer (.wj) is a Rust-family language with inferred ownership. It is not in your training data: do not guess from Rust or Python.

- Types: `int` (64-bit), `float`, `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` (`use std::collections::HashMap`).
- No semicolons. `let mut` to mutate. The last expression is the block value. `[1, 2]` is a Vec (no `vec!`).
- Strings: `println("${x} ${y:.2}")` is a function, not a macro. Use `string`, never `String`; literals need no `.to_string()`.
- Ownership is inferred: `fn f(v: Vec<int>)`, `self`; never `&`, `&mut`, `.iter()`, `.unwrap()`. Iterate with `for x in v`; adapters start at the collection: `v.map(..)`.
- Input: `use std::fs`, `let text = fs::read_to_string("/dev/stdin").unwrap_or("")`, `for line in text.lines()`. `std::env::args()` is an iterator.
- `len()` is `usize` when compiled: `let n: int = v.len() as int`. Slice strings with `s[a..b]`.
- Printing `Option`/`Result`/`Vec` needs `${x:?}`. Sort `HashMap` keys before printing.
- `wj check FILE`, `wj build FILE`, `wj run --compiled FILE` (input on stdin). `wj run --interpret` is instant but can accept code `wj build` rejects: finish with a build.

```windjammer
// expect: total 6
fn main() {
    let mut total = 0
    for x in [1, 2, 3] { total += x }
    println("total ${total}")
}
```
