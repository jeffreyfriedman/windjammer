# Compiled: an owned handle forwarded to a generic function is inferred borrowed and cloned

```windjammer
pub struct Handle<T> {
    items: Vec<T>,
}

pub fn first<T>(h: Handle<T>) -> (Handle<T>, Option<T>) {
    (h, None)
}

pub fn first_int(h: Handle<int>) -> (Handle<int>, Option<int>) {
    first(h)
}

pub fn drain(mut h: Handle<int>) -> int {
    let mut n = 0
    while n < 3 {
        let got = first_int(h)
        h = got.0
        n = n + 1
    }
    n
}
```

`wj build` emitted `first_int(h: &Handle<i64>)` with `first(h.clone())` and `drain(h: &Handle<i64>)` with `h = got.0`. For a type that is not
`Clone` (the ecosystem `Receiver<T>` wrapper over `mpsc::Receiver`) that is `E0599`/`E0308`.

Causes and fixes:
- The passthrough rule compared the callee formal `Handle<T>` with the caller's `Handle<int>` for equality, so a generic callee never counted as an
  owning forward. A callee type parameter now matches any argument (`generic_instance_of` in `analyzer/parameter_analysis.rs`); the bare-pass
  restore (`multipass_bare_pass_demotion`) treats a generic user type like a plain `Custom` one.
- A parameter that is assigned as a whole (`h = step(h)`) is an owned `mut` binding, not a `&mut` the caller observes (`Analyzer::is_rebound`, rule
  in `infer_parameter_ownership`). Copy parameters are unchanged.

Status: fixed. Gates: `ecosystem_findings2::owned_handle_threaded_through_generic_wrappers_is_moved`, `owned_handle_forward_does_not_require_clone`.
Unblocks the ecosystem `wj-sync` package and the `wj-pipeline` app.