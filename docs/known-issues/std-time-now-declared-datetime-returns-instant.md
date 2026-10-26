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

Status: fixed. `std::time::now()` is `DateTime` in `std/time.wj`, the runtime and the interpreter; the monotonic clock is `time::instant_now()` in the runtime. Gate: `ecosystem_findings_test::time_now_declared_type_matches_runtime_type`.
