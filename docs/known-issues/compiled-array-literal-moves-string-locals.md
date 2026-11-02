# Compiled path: an array literal of `string` locals moves them, so reusing a name inside the literal or after it fails in rustc

Found while building `scripts/wj/bench_effects_overhead.wj` with `wj run --compiled`: the arm loop `for arm in [base, test, test, base]`
names each string twice and reads both again after the loop.

```windjammer
fn main() {
    let base: string = "off"
    let test: string = "on"
    let mut r = 0
    while r < 2 {
        for arm in [base, test, test, base] {
            println("${arm}")
        }
        r += 1
    }
    println("${base} ${test}")
}
```

`wj run --interpret` prints the arms and the final line. `wj build` fails with `error[E0382]: use of moved value: test` and
`base` (inside the literal, on the second pass of the loop, and in the last `println`); rustc suggests cloning.
Workaround: index a list of names or write `base.clone()` at each use.
Fix location: the ownership pass should treat an element of an array literal like a by-value argument, and a use of the same
name later in the literal, later in the loop body, or after the loop as a reason to clone at the earlier use sites (the same rule
the pass applies to a call with two by-value uses of one local).
Guard: add the program above to the compiled-versus-interpreted corpus once it is fixed.
