# Migrating to explicit `let mut` (ADR-0002)

Local bindings are immutable unless declared `mut`. A local the program changes without `mut` (assigned, a field or
element assigned, a method that changes it such as `push`, or passed where the callee changes it) is the error
`WJ0051`, reported the same way by `wj check`, `wj build` and `wj run`. This includes loop variables: a loop that
assigns to its variable or changes it through declares it `mut` (`for mut e in entities { e.x = 0 }`,
`for (id, mut qty) in items { qty = qty + 1 }`); the loop still writes through to the collection, and which borrow it
takes is still inferred. Parameters and `self` are unaffected: how they are passed is still inferred.

## Moving a project to this compiler

1. Bump the project's compiler pin to a build that includes the change (`wj doctor` shows the running build).
2. Run `wj fix <project-dir>` (the directory with `wj.toml`). It inserts every missing `mut` and prints one line per
   binding (`src/app.wj:12: \`items\` is changed: \`mut\` added (WJ0051)`). `wj fix --check <dir>` lists them without
   writing and exits 1 while any is missing; `wj fix --docs` also fixes ```` ```wj ```` fences in Markdown.
3. Run the project's tests interpreted and compiled. `wj check` must report no `WJ0051`.

`wj fix` also migrates deprecated surface syntax. To add only the `mut`s, use the repository script:
`wj run --allow fs.read=DIR/** --allow fs.write=DIR/**/*.wj scripts/wj/add_missing_mut.wj -- <wj> DIR`.

Each fix is the edit `wj check --json` publishes for the diagnostic (`"replacement": "mut "`), so editors and agents can
apply the same fix one diagnostic at a time.
