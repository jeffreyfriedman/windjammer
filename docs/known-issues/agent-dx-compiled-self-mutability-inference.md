# Compiled path: `self` mutability inference misses trait impls, recursion and calls through `?`

Windjammer infers `&self` / `&mut self` from the body and rejects explicit `&mut self` (lint W0001). Three cases are inferred wrongly.

1. `impl Iterator for X { fn next(self) -> Option<i64> { self.n += 1; ... } }` is emitted as `fn next(&self)`: E0053 (the trait needs
   `&mut self`) and E0594 (cannot assign behind `&`). A custom iterator type can therefore not be written; a plain method
   (`advance`) works. (t039)

```windjammer
struct Counter { n: i64 }

impl Iterator for Counter {
    type Item = i64
    fn next(self) -> Option<i64> {
        self.n += 1
        if self.n > 3 { None } else { Some(self.n) }
    }
}
```

2. A recursive method that mutates `self.left[n] = self.remove_at(self.left[n], k)` is emitted with an immutable `self` (E0596). Writing
   `&mut self` is rejected by the leakage lint. The benchmark's BST delete is iterative instead. (t013)

3. A method whose only mutation is `self.change(from, -amount)?` (a call to another mutating method wrapped in `?`) is emitted as `&self`
   (E0596 "cannot borrow `*self` as mutable"); inlining the mutation fixes it. (t030)
