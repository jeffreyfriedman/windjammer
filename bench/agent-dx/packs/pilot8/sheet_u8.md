# Windjammer for agents

Windjammer (.wj) is a Rust-family language with inferred ownership. It is not in your training data: do not guess from Rust or Python.

- Types: `int` (64-bit), `float`, `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>` (`use std::collections::HashMap`).
- No semicolons. `let mut` to mutate. The last expression is the block value. `[1, 2]` is a Vec (no `vec!`).
- Strings: `println("${x} ${y:.2}")` is a function, not a macro. Use `string`, never `String`; literals need no `.to_string()`. A dollar sign before a value is `"\$${p}"` (`$$` prints `$` and stops the interpolation).
- Ownership is inferred: `fn f(v: Vec<int>)`, `self`; never `&`, `&mut`, `.iter()`, `.unwrap()`. Iterate with `for x in v`; adapters start at the collection: `v.map(..)`.
- Input: `use std::fs`, `let text = fs::read_to_string("/dev/stdin").unwrap_or("")`, `for line in text.lines()`. `std::env::args()` is an iterator.
- `len()` is `usize` when compiled: `let n: int = v.len() as int`. Slice strings with `s[a..b]`.
- JSON: `use std::json`; any struct converts: `json.to_string(v)`, `json.parse_string::<T>(text)` (no derive).
- Printing `Option`/`Result`/`Vec` needs `${x:?}`. Sort `HashMap` keys before printing.
- Rust crates: add `strsim = "0.11"` under `[dependencies]` in `wj.toml`, call `strsim::levenshtein(a, b)`; `wj rust-surface strsim` lists what is callable.
- Unsure? `wj docs <terms>` searches the full reference, stdlib and verified examples; `wj explain <CODE>` explains an error code.

Idioms:
```windjammer
// expect: 3 b
// expect: 6 9
use std::process
enum E { N(int), Add(Box<E>, Box<E>) }   // recursive: Box, `*a` opens it
fn ev(e: E) -> int { match e { E::N(n) => n, E::Add(a, b) => ev(*a) + ev(*b) } }
trait Sh { fn area(&self) -> int }   // `dyn`: Box<dyn Sh>
struct Sq { s: int }
impl Sh for Sq { fn area(self) -> int { self.s * self.s } }
struct Acc { t: int }
impl Acc { fn add(self, x: int) { self.t += x } }   // changes self: the caller writes `let mut`
fn main() {
    let p: Vec<string> = "a b c".split(" ").collect()   // iterators: collect before p[0], p.len()
    println("${p.len()} ${p[1]}")
    let mut v: Vec<string> = Vec::new()
    for l in "x\ny".lines() { v.push(l.to_string()) }   // borrowed item -> owned (never on a literal)
    let s: Vec<Box<dyn Sh>> = [Box::new(Sq { s: 3 })]
    let mut a = Acc { t: 0 }
    a.add(s[0].area())
    println("${ev(E::Add(Box::new(E::N(1)), Box::new(E::N(5))))} ${a.t}")
    if v.len() != 2 { process::exit(1) }   // exit status; end of main is 0, a panic 101
}
```
