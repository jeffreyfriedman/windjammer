# Compiled: using `std::json` anywhere makes every struct in the crate derive `Serialize`/`Deserialize`

A crate (`wj.toml` with `wj_rate_limit = { path = "../../packages/wj-rate-limit/build" }`):

```windjammer
// a.wj
use std::json

pub struct Ok {
    pub ok: bool,
}

pub fn health() -> string {
    json.to_string(Ok { ok: true }).unwrap_or("")
}
```

```windjammer
// b.wj  (never serialized)
use std::collections::HashMap
use wj_rate_limit::WindowBucket

pub struct Limiter {
    pub buckets: HashMap<string, WindowBucket>,
}
```

`b.rs` gets `#[derive(Debug, Clone, Serialize, Deserialize)]` on `Limiter` and rustc fails with
`WindowBucket: serde::Serialize is not satisfied`. Derives should follow what is actually serialized (or be limited to
structs that reach a `json` call), otherwise any struct holding a dependency type breaks the build. Moving the `json`
use out of the crate fixes it, which is not a reasonable fix for an app.
