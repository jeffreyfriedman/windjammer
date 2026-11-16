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
- Loop: one command, `wj run --verify --stdin-file in.txt FILE` (input: `--stdin-file in.txt`), runs the program, builds it with the production compiler, runs the build and compares. It prints `wj verify: OK` and exits 0 when both runs agree, even if your program exits 1 on purpose (the line names its status); otherwise fix what it reports (one line per problem: `file:line:col: error CODE: message | help | fix`) and re-run.
- Rust crates: add `strsim = "0.11"` under `[dependencies]` in `wj.toml`, call `strsim::levenshtein(a, b)`; `wj rust-surface strsim` lists what is callable.
- One command, no redirects, no chains (`2>&1`, `|`, `&&`, `;`, `$?`, `${PIPESTATUS}`): the tool already merges stderr and prints the exit status (last line `exit status: N`).
- Input: write it to a file with the Write tool, pass `--stdin-file in.txt` (never `printf`, `$()` or a heredoc; a `printf` argument starting with `=` is refused). `wj build` prints `run: ...`; there is no `./build/main`.
- Style: dispatch with `match` on strings and enums, not if-chains; the last expression is the value (no `return`); no `let` per sub-expression.
- Already exists, never hand-roll: `x.abs()`, `a.min(b)`, `a.max(b)`, `s.contains("t")`, `s.parse::<int>()`, `c.is_alphabetic()`, `c.is_ascii_digit()`, `[0; n]`.
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

```windjammer
// expect: 2 5 -5 3 true 42 true 3
fn kind(cmd: string) -> int {
    match cmd { "add" => 1, "neg" => 2, _ => 0 }
}
fn main() {
    let (a, b): (int, int) = (-5, 3)
    let n: int = "42".parse().unwrap_or(0)
    println("${kind("neg")} ${a.abs()} ${a.min(b)} ${a.max(b)} ${"hello".contains("ell")} ${n} ${'a'.is_alphabetic()} ${[0; 3].len()}")
}
```
