# Compiled: `tx.send(a + b)` on a `Sender<int>` emits `a + b.to_string()`

```windjammer
use std::sync::mpsc

fn send_sum(a: int, b: int) -> int {
    let pair = mpsc::channel()
    let tx = pair.0
    let rx = pair.1
    match tx.send(a + b) {
        Ok(_) => {},
        Err(_) => {},
    }
    match rx.recv() {
        Ok(v) => v,
        Err(_) => 0,
    }
}

fn main() {
    println("${send_sum(1, 2)}")
}
```

`wj build` emits `tx.send(a + b.to_string())` (`E0277: cannot add String to i64`; the same appears for `v * 2` and `-1`).
`send` is treated as a string-argument method and a `.to_string()` is appended to the last operand instead of the whole
expression; for a channel of `int` no conversion should be added at all.
