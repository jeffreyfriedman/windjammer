# Ownership inference: free functions with the same name in different modules share parameter modes

`src/domain/a.wj`:

```windjammer
pub struct User {
    pub id: int,
}

pub fn show(user: User, n: int) -> int {
    user.id + n
}
```

`src/domain/b.wj`:

```windjammer
use crate::domain::a::User

pub fn show(user: User, sink: Vec<User>) -> int {
    sink.push(user)
    sink.len()
}
```

`src/domain/c.wj` calls both (`use ...a::show as show_user`, `use ...b::show as show_sink`).

`wj build` emits `a::show(user: User, n: &mut i64)` (the `&mut` belongs to `b::show`'s `sink`) and the callers disagree
with the callees (`E0277: cannot add &mut i64 to i64`, `E0308`). Renaming one function (`show_sink`) makes the build pass,
so the signature registry is keyed by the bare function name rather than by module path / `DefId`. A real app hits this
with `list`, `show`, `create`, `remove` in several handler modules. Related: the comment in `wj-notes-api` about
`query_get` stealing ownership metadata.

Status: handed to the convergence stream (registry keyed by bare function name). Ratchet: `ecosystem_findings_handoff_test::same_name_free_functions_keep_own_parameter_modes`, listed in `.config/known-test-failures.txt`.
