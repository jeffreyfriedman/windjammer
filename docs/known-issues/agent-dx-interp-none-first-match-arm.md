# Interpreter: `None` as the first match arm matches every value

```windjammer
fn main() {
    let o: Option<i64> = Some(5)
    let a = match o {
        None => 0,
        Some(b) => b,
    }
    println("${a}")
}
```

`wj run --interpret` prints `0` (compiled Rust prints `5`). With the arms swapped (`Some` first) it is correct. Found by
`bench/agent-dx` task t035 (generic min/max stack) and t057. Fixed on the interp-gaps branch ("None-pattern fix"); present in
binaries built from earlier trees. Agents write `None` first often, so this is silent wrong output for a common shape.
