# Windjammer for agents

Windjammer (.wj) is a Rust-family language with inferred ownership. It is not in your training data: do not guess.

- Types: `int`, `float`, `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` (`use std::collections::HashMap`).
- No semicolons. `let mut` to mutate. The last expression is the block value. `[1, 2]` is a Vec (no `vec!`).
- Strings: `println("${x} ${y:.2}")` is a function. Use `string`, never `String`; literals need no `.to_string()`. A dollar sign before a value is `"\$${p}"`.
- Ownership is inferred: `fn f(v: Vec<int>)`, `self`; never `&`, `&mut`, `.iter()`, `.unwrap()`. Iterate with `for x in v`; adapters start at the collection: `v.map(..)`. The one `&` you write is the receiver of a bodiless trait method (`fn put(&mut self, k: int)`, W0016).
- `split`, `lines`, `chars` give an iterator, not a list: index, `.len()` or `.contains(..)` need `let parts: Vec<string> = s.split(" ").collect()` first (WJ0015, WJ0035).
- Input: `use std::fs`, `fs::read_to_string("/dev/stdin").unwrap_or("")`, `for line in text.lines()`.
- `len()` is `usize` when compiled: `let n: int = v.len() as int`. Slice strings with `s[a..b]`.
- JSON: `use std::json`; `json.to_string(v)`, `json.parse_string::<T>(text)` (no derive).
- Printing `Option`/`Result`/`Vec` needs `${x:?}`. Sort `HashMap` keys before printing.
- Loop: one command, `wj run --verify FILE` (stdin or `--stdin-file F`). It checks (`wj check`), interprets (`wj run --interpret`), stops on a panic, then builds once (`wj build` must succeed) and compares the compiled run (`wj run --compiled`). Done at `wj verify: OK`.
- Rust crates: `strsim = "0.11"` under `[dependencies]` in `wj.toml`, call `strsim::levenshtein(a, b)`; `wj rust-surface strsim` lists its API.
- Unsure? `wj docs <terms>` searches the reference, stdlib and examples; `wj explain <CODE>` explains an error.

```windjammer
// expect: total 6
fn main() {
    let mut total = 0
    for x in [1, 2, 3] { total += x }
    println("total ${total}")
}
```
