# Interpreter: a loop variable is not written back to its element

A loop that changes its variable writes through to the collection (ADR-0002 addendum: the variable is declared `mut`,
the borrow is inferred). The compiled backend does this (`for p in &mut ps { p.x += 10 }`); the interpreter binds a copy,
so the change is lost:

```windjammer
struct P {
    x: int,
}

fn main() {
    let mut ps = [P { x: 1 }]
    for mut p in ps {
        p.x = p.x + 10
    }
    println("${ps[0].x}")
}
```

Compiled: `11`. Interpreted: `1`. Same for a tuple element (`for (id, mut qty) in self.items { qty = qty + n }`).
The fix belongs in the interpreter's `for` (bind the element as a place when the variable is declared `mut`).
Test: `tests/local_mutability_test.rs::mut_loop_variables_write_through_in_the_generated_rust` (compiled side).
