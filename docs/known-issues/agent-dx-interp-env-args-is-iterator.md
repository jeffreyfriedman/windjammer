# Interpreter: `std::env::args()` is an iterator, not a `Vec<string>`

```windjammer
use std::env

fn main() {
    let a = env::args()
    println("${a.len()}")
    let b = a[1..].to_vec()
    println("${b.len()}")
}
```

`a.len()` works but `a[1..]` fails with "no method named `slice` found for value of type `Iterator`"; compiled
`windjammer_runtime::env::args()` returns `Vec<String>`. `let a: Vec<string> = env::args().collect()` works in both modes
(used by the benchmark solutions). Also: `wj run --interpret main.wj` followed by a program flag such as `--name=x` makes the `wj` CLI reject the program's own flag;
programs need a `--` separator before their own flags: `wj run --interpret main.wj` then `--` then the flags.
