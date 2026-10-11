# Reading standard input: no single API works in both the interpreter and the compiled path

| Form | `wj run --interpret` | compiled (`wj build`) |
|------|----------------------|-----------------------|
| `io::stdin().read_line(line)` (loop until 0) | works | **never terminates**: transpiled to `read_line(line.clone())`, the clone is filled, `line` stays empty |
| `io::read_line()` | "not implemented by the interpreter" | works |
| `io::read_to_string(io::stdin())` | "not implemented by the interpreter" | works |
| `fs::read_to_string("/dev/stdin")` | works | works (Unix only) |

`&mut line` is rejected by the leakage lint (W0004), so the explicit-borrow spelling is not available. The benchmark solutions use
`fs::read_to_string("/dev/stdin")`; an agent that tries the first form gets an infinite loop in the compiled path. Repro:

```windjammer
use std::io

fn main() {
    let mut line = String::new()
    let n = match io::stdin().read_line(line) { Ok(k) => k, Err(_) => 0 }
    println("${n}")
}
```
