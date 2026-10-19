---
name: windjammer-closure-shorthand
description: Use when writing `_` closures in Windjammer nested calls.
category: windjammer-language
---

# `_` binds to the smallest enclosing call argument

- `_` becomes a closure around the smallest call whose argument contains it. In `group_by(key(_.clone()))` the closure wraps `key(...)`, not `group_by`. The compiler now rejects this at parse time when the callee is declared in the same file and the receiving parameter is certainly not callable (a number, string, list, option, result, tuple, or a reference to one). The error explains the scoping rule and shows the explicit closure to write. Callees in other files, the standard library and methods are not checked, so the bad form can still compile and misbehave there.
- Inside a nested call, write the closure explicitly: `group_by(|u| key(u.clone()))`.
- A bare `_` is valid only as the placeholder in a `|>` stage. Elsewhere it is an error.