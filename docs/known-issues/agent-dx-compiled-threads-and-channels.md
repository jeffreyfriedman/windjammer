# Compiled path: `std::thread` / `mpsc` programs need several workarounds; a channel pipeline cannot be written

Found by t042-t046. Natural form and the first error each hits:

```windjammer
use std::thread

fn main() {
    let mut handles = Vec::new()
    let mut i: i64 = 0
    while i < 3 {
        let k = i
        let h = thread::spawn(move || {
            let s = k * 10
            s
        })
        handles.push(h)
        i += 1
    }
    for h in handles {
        println("${h.join().unwrap_or(0)}")
    }
}
```

1. `handles.push(h)` becomes `handles.push(h.clone())`: E0599, `JoinHandle` is not `Clone`.
2. `let mut handles = Vec::new()` is emitted as `Vec<JoinHandle>` (E0425 not in scope / E0107 missing generics); an explicit
   `Vec<JoinHandle<i64>>` annotation plus `use std::thread::JoinHandle` is needed.
3. When the `spawn` call is itself an argument (`handles.push(thread::spawn(move || { ...; s }))`), the closure's tail expression is
   emitted as `let _ = s;`, so the handle is `JoinHandle<()>` (E0308). An explicit `return s` in the closure fixes it.
4. `k * 10` inside the closure becomes `k * 10_i32` against an `i64` (E0308/E0277).
5. A `Receiver` passed to a helper and moved into a thread (`fn apply(stage: string, rx: Receiver<i64>) -> Receiver<i64>` with
   `thread::spawn(move || { loop { rx.recv() ... } })`) is inferred as a borrowed parameter: E0277 "`Receiver<i64>` cannot be shared
   between threads safely". Moving it in the caller instead (`let inbox = rx`) is emitted as `rx.clone()` (E0599). With these two,
   a multi-stage channel pipeline (t043) does not compile in any spelling tried; this is the one benchmark task that no workaround rescued.
6. `std::sync::SharedInt` (the "no Arc/Mutex in WJ source" vocabulary) is not `Clone`, so it cannot be handed to several threads; the
   histogram (t044) uses `Arc<Mutex<Vec<i64>>>` with `match h.lock() { Ok(mut g) => .., Err(_) => {} }` because `.unwrap()` is a lint error.

The interpreter does not support threads or channels at all (documented, loud).
