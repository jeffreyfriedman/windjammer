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

Status: does not reproduce on next/v0.50.0 (`tx.send(a + b)`, `v * 2`, `-1` on an `int` channel all emit the argument unchanged; a `string` channel still converts literals). Gates: `ecosystem_findings2::sender_send_arithmetic_on_int_channel`, `sender_send_string_channel_still_converts_literals`. The error `wj-sync` still showed was a different class: an owned `Receiver<int>` threaded through generic wrappers was inferred `&Receiver` and forwarded as `recv(rx.clone())` (`Receiver` is not `Clone`); see `compiled-owned-handle-forwarded-through-generic-wrapper-is-cloned.md`.
