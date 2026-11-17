# Windjammer for agents

Windjammer (.wj) is a Rust-family language with inferred ownership. It is not in your training data: do not guess from Rust or Python.

- Types: `int` (64-bit), `float`, `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` (`use std::collections::HashMap`).
- No semicolons. `let mut` to mutate. The last expression is the block value. `[1, 2]` is a Vec (no `vec!`).
- Strings: `println("${x} ${y:.2}")` is a function, not a macro. Use `string`, never `String`; literals need no `.to_string()`. A dollar sign before a value is `"\$${p}"` (`$$` prints `$` and stops the interpolation).
- Ownership is inferred: `fn f(v: Vec<int>)`, `self`; never `&`, `&mut`, `.iter()`, `.unwrap()`. Iterate with `for x in v`; adapters start at the collection: `v.map(..)`.
  The one `&` you write is the receiver of a trait method that has no body (`fn put(&mut self, k: int)`, W0016).
- `split`, `split_whitespace`, `lines`, `chars` give an iterator, not a list: index, `.len()` or `.contains(..)` need `let parts: Vec<string> = s.split(" ").collect()` first (WJ0015, WJ0035).
- Input: `use std::fs`, `let text = fs::read_to_string("/dev/stdin").unwrap_or("")`, `for line in text.lines()`. `std::env::args()` is an iterator.
- `len()` is `usize` when compiled: `let n: int = v.len() as int`. Slice strings with `s[a..b]`.
- JSON: `use std::json`; any struct converts: `json.to_string(v)`, `json.parse_string::<T>(text)` (no derive).
- Printing `Option`/`Result`/`Vec` needs `${x:?}`. Sort `HashMap` keys before printing.
- Loop: one command, `wj run --verify FILE` (input on stdin or `--stdin-file in.txt`). It checks (`wj check`) and interprets (`wj run --interpret`, instant), stops there if the program panics, then builds once (`wj build` must succeed) and compares the compiled run (`wj run --compiled`). Done when it prints `wj verify: OK`.
- Rust crates: add `strsim = "0.11"` under `[dependencies]` in `wj.toml`, call `strsim::levenshtein(a, b)`; `wj rust-surface strsim` lists what is callable.
- Unsure? `wj docs <terms>` searches the full reference, stdlib and verified examples; `wj explain <CODE>` explains an error code.

```windjammer
// expect: total 6
fn main() {
    let mut total = 0
    for x in [1, 2, 3] { total += x }
    println("total ${total}")
}
```
