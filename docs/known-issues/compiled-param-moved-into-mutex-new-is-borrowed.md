# Compiled: a parameter moved into `Mutex::new` is inferred as a borrow

```windjammer
use std::sync::{Arc, Mutex}

struct Counter {
    n: int,
}

fn share(c: Counter) -> Arc<Mutex<Counter>> {
    Arc::new(Mutex::new(c))
}

fn main() {
    let s = share(Counter { n: 1 })
    println("done")
}
```

`wj build` infers `c: &Counter` and emits `Arc::new(Mutex::new(c))` (`E0308: expected Counter, found &Counter`; rustc suggests
`c.clone()`). A parameter that is moved into a constructor/container must stay owned. The same shape appears in a server
entry point (`fn serve(port: int, app: App)` that wraps `app` in `Arc<Mutex<..>>`).

Status: handed to the convergence stream. Ratchet: `ecosystem_findings_handoff_test::param_moved_into_mutex_new_stays_owned`, listed in `.config/known-test-failures.txt`. Blocks the ecosystem `wj-taskflow` app (`fn serve(port: int, app: Taskflow)` emitted as `app: &Taskflow`).
