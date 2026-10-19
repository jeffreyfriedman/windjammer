---
name: windjammer-closure-shorthand
description: Use when writing `_` closures in Windjammer nested calls.
category: windjammer-language
---

# `_` binds to the smallest enclosing call argument

- `_` becomes a closure around the smallest call whose argument contains it. In `group_by(key(_.clone()))` the closure wraps `key(...)`, not `group_by`. The program then fails at runtime.
- Inside a nested call, write the closure explicitly: `group_by(|u| key(u.clone()))`.
- A bare `_` is valid only as the placeholder in a `|>` stage. Elsewhere it is an error.