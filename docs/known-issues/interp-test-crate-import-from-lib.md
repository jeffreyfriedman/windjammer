# `wj test --interpret`: `use crate::item` in `tests/` is not resolved against src/lib.wj

Package layout:

```
wj.toml            # [package] + [lib]
src/lib.wj         # pub fn add(a: int, b: int) -> int { a + b }
tests/a_test.wj
```

```windjammer
use crate::add

@test
fn test_add() {
    assert_eq(add(1, 2), 3)
}
```

`wj test` (compiled) passes. `wj test --interpret` fails with
`unsupported in the interpreter: crate::add is not implemented by the interpreter`, so every package-shaped project
(src/lib.wj plus `tests/*_test.wj`, the ecosystem convention) is only testable compiled. Expected: the interpreter loads
src/lib.wj as the crate root and binds `crate::` paths in test files to it, as it does for `wj run` projects.
