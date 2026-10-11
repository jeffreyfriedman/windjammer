# Compiled path: miscellaneous codegen/inference bugs found by agent-dx

1. **A user method named `collect` is rewritten to the iterator form.** `t.collect(out)` becomes `t.collect::<Vec<_>>(&mut out)`
   (E0107). Any user method that shares an iterator method name is at risk. (t013)

2. **`return if c { "a" } else { "b" }` inside an `if` block drops the value.**

```windjammer
fn pick(flag: bool) -> string {
    if flag {
        return if flag { "yes" } else { "no" }
    }
    "none"
}
```

   emits `return { if flag { let _ = String::from("yes"); } else { let _ = String::from("no"); } }` (E0308: expected `String`, found `()`).
   Binding the `if` to a `let` first works. (t032)

3. **A variable from a `Receiver` loop is treated as a reference.** In `for x in rx { prev = x }` and `x == prev` the generated Rust has
   `*x` (E0614: `i64` cannot be dereferenced). Pulling values with `match rx.recv()` or comparing through subtraction avoids it. (t043)

4. **Type inference of an unannotated binding from a later index expression** produced `Vec<usize>` for the result of a function
   returning `Vec<i64>` (`let mut v = to_vec(list)` then `v[i]`), E0308 (t015, first draft; annotation fixes it).
