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

4. A method that calls a mutating method only from a `return` inside a `loop` is inferred `&mut self`, but its own caller is
   emitted as `&self` (E0596): the mutation does not propagate one more level. Found with the `wj-redis` example of ADR-0032
   (`TestServer::serve_client` -> `serve_one` -> `execute`); `wj build s.wj --no-cargo` shows `fn step(&mut self)` and
   `fn serve_client(&self)` with the base compiler and with the ADR-0032 branch alike.

```windjammer
struct Srv { hits: Vec<int> }

impl Srv {
    fn record(self, v: int) -> int {
        if v > 0 {
            self.hits.push(v)
            return 1
        }
        0
    }

    fn step(self) -> bool {
        loop {
            return self.record(1) > 5
        }
    }

    fn serve_client(self) -> int {
        let mut n = 0
        while self.step() {
            n = n + 1
        }
        n
    }
}
```
