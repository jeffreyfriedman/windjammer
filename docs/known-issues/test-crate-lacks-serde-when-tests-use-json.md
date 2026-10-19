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

Status: fixed. The generated test crate declares the crates its modules import (`serde` with `derive`, and library dependencies from the library `Cargo.toml`). Gates: `ecosystem_findings_test::package_test_using_json_has_serde_in_test_crate`, `test_runner::test_execution::test_crate_dependency_tests`.
