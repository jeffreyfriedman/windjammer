# Rust backend: a by-value generic parameter consumed by `.into()` is inferred as a borrow

```windjammer
fn label<T: Into<string>>(t: T) -> string {
    t.into()
}
```

Generated Rust takes `t: &T`, so `t.into()` needs `String: From<&T>` (E0277). Ownership inference
does not treat `Into::into` (a `self`-consuming trait method on a generic parameter) as a consuming
use. Input for the inference convergence stream.

Ratchet: `tests/conformance_programs/164_into_bound_param_consumed` is listed in `SUPPORT.toml` for
`rust` and must FAIL there until the inference is fixed (the harness fails when it starts passing);
it passes on the interpreter and the portable backends.
