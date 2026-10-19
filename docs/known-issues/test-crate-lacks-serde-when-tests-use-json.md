# `wj test`: a test file that uses `std::json` fails with `unresolved import serde`

`tests/x_test.wj`:

```windjammer
use std::json

struct Body {
    token: string,
}

@test
fn test_parse() {
    let b: Body = json.parse_string("{\"token\":\"t\"}").unwrap()
    assert_eq(b.token, "t")
}
```

The generated test module starts with `use serde::{Serialize, Deserialize};` but the generated test crate has no `serde`
dependency (`E0432: unresolved import serde`). The library crate gets `serde` automatically when it uses `std::json`; the
test crate should too.
