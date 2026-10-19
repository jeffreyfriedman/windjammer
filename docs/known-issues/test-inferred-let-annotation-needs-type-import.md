# Compiled tests: an un-annotated `let` gets an emitted type annotation naming a type the test file never imported

src/lib.wj:

```windjammer
pub struct Rule {
    pub name: string,
}

pub fn make() -> Vec<Rule> {
    let mut out = Vec::new()
    out.push(Rule { name: "a" })
    out
}
```

tests/rules_test.wj:

```windjammer
use crate::make

@test
fn test_make() {
    let rules = make()
    assert_eq(rules.len(), 1)
}
```

`wj test` emits `let rules: Vec<Rule> = make();` in the generated test module and fails with `E0425: cannot find type Rule
in this scope` (rustc suggests `use <crate>::Rule`). The source never names `Rule`, so it should not need to import it.
Adding `use crate::Rule` to the test makes it pass, which is the only workaround. Expected: the annotation is only
emitted when the type is in scope (or is path-qualified).

The same happens in non-test code: `let rules = parse(text)` where `parse` comes from another package
(`wj_gitignore::parse` returning `Vec<Rule>`) is emitted as `let rules: Vec<Rule>` without `use wj_gitignore::Rule`.
