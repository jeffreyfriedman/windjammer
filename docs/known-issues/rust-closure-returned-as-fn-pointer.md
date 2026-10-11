# Rust target: a capturing closure returned as `fn(..)` is not accepted by rustc

```windjammer
fn make_adder(n: i64) -> fn(i64) -> i64 {
    |x: i64| x + n
}
```

Windjammer's `fn(i64) -> i64` return type is emitted as a Rust fn pointer, but a capturing closure is not coercible
(`E0308: expected fn pointer, found closure`). Affected: corpus program 63 (also passing a capturing closure to a
`fn(..)` parameter, program 17).
