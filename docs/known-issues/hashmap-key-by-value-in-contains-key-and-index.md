# HashMap<i64, i64>: `contains_key(&k)` / `map[&k]` lose the reference

Repro (`repro.wj`):

```
use std::collections::HashMap

fn main() {
    let mut counts: HashMap<i64, i64> = HashMap::new()
    let mut x: i64 = 12345
    for i in 0..10 {
        x = x ^ (x << 13)
        let k = x % 16
        if counts.contains_key(&k) {
            let c = counts[&k]
            counts.insert(k, c + 1)
        } else {
            counts.insert(k, 1)
        }
    }
    println!("{}", counts.len())
}
```

Generated Rust contains `counts.contains_key(k)` and `counts[*(&k) as usize]`
(rustc: expected `&i64`, found `i64` / `usize`). Found while writing the
`benchmarks/rust_parity` kernels; the HashMap kernel was replaced by a Vec
histogram.

Same session also showed: an unsuffixed `for n in 1..N` loop whose variable is
passed to a `u64` formal infers `i32` for the range (rustc E0308), and
`let mut e = 0.0` accumulating `f64` fields infers `f32`; the kernels annotate
types to avoid both.
