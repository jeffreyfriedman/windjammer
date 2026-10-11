# Compiled path: `cmp` arguments lose their borrow; `x[a..b]` / `x[a..]` use `i32` literal bounds

```windjammer
struct P { lo: i64 }

fn main() {
    let mut v: Vec<P> = [P { lo: 2 }, P { lo: 1 }]
    v.sort_by(|a, b| a.lo.cmp(&b.lo))
    let s = "hello world"
    let t = s[6..]
    println(t)
}
```

Generated Rust:

```rust
v.sort_by(|a, b| a.lo.cmp(b.lo));          // E0308: expected `&i64`, found `i64`  (the `&` was dropped)
let t = &s[6_i32..s.len()];                // E0308 + E0277: the range mixes i32 and usize
```

The explicit borrow in `a.lo.cmp(&b.lo)` is stripped (and writing it without `&` fails the same way), so there is no spelling of a
`sort_by` comparator on a primitive field that compiles; the benchmark sorts with hand-written insertion sorts. Slicing with a
literal start bound fails for `Vec`, `String` and `&str`; the benchmark uses `chars().skip(a).take(b - a).collect::<string>()` helpers.
Also seen: `vec![0, 0]` is `Vec<i32>` even where a `Vec<i64>` is required (t044) and `k * 10` inside a closure over an `i64`
becomes `k * 10_i32`. Related: [rust-int-literal-width-inference](rust-int-literal-width-inference.md).
