# Compiled path: string literals and `Option` defaults are mis-typed in generated Rust

Found by `bench/agent-dx` (first-draft solutions to t009, t010, t011, t014, t018, t025, t027, t030, t031, t033, t054, t055).

```windjammer
use std::collections::HashMap

fn main() {
    let text = "a b"
    for tok in text.split(" ") {
        if tok == "a" {
            println("a")
        }
    }
    let mut it = "5\n6".lines()
    let first = it.next().unwrap_or("0")
    let mut m: HashMap<string, i64> = HashMap::new()
    m.insert("a", 1)
    let n = m.get("a").unwrap_or(0)
    println("${first} ${n}")
}
```

Generated Rust (`wj build --no-cargo`):

```rust
if *tok == "a" {                                   // E0277: can't compare `str` with `&str`
let first = it.next().unwrap_or(String::from("0")); // E0308: expected `&str`, found `String`
let n = m.get("a").unwrap_or(0_i32);               // E0308: expected `&i64`, found `i32`
```

All three are the most idiomatic ways to write these lines, and the interpreter runs them correctly. Workarounds used in the
benchmark: collect `split` results into `Vec<string>` first, read all lines into a `Vec<string>` and index, and replace
`get(..).unwrap_or(d)` with a helper that loops or a `match` that does not mix `&V` and `V`.
