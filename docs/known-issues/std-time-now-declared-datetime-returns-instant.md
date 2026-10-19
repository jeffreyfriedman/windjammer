# `std::time::now()` is declared `-> DateTime` in `std/time.wj` but the runtime returns an `Instant`

```windjammer
use std::time

fn main() {
    println("${time.now().timestamp_millis()}")
}
```

`wj build` fails with `E0599: no method named timestamp_millis found for struct windjammer_runtime::time::Instant`.
The declaration says `now() -> DateTime` (which has `timestamp_millis`), the runtime function `now()` returns `Instant`.
`time.utc_now().timestamp_millis()` works. One of the two sides should change so the declared and emitted types agree.
