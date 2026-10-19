# Compiled: `fs.read_to_string(path)` with a `string` parameter is emitted as a method call on a nonexistent `fs` value

```windjammer
use std::fs

fn size(path: string) -> int {
    match fs.read_to_string(path) {
        Ok(t) => t.len(),
        Err(_) => 0,
    }
}

fn main() {
    println("${size("Cargo.toml")}")
}
```

`wj build` emits `match fs.read_to_string(&mut path)` (rustc `E0423: cannot find value fs`). The same shape with `fs.write`
or `fs.is_file` is emitted correctly as `fs::write(&path, ..)`. Reproduced on next/v0.50.0 (a build after the
ownership/`&mut self` work); the ecosystem `wj-todo-cli` adapter uses exactly this shape. Expected `fs::read_to_string(&path)`.
