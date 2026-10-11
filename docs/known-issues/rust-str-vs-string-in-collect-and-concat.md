# Rust target: `&str` / `String` mismatches in collect and concatenation

* `let words: Vec<string> = "  x   y  ".split_whitespace().collect()` emits `Vec<String>` collected from `&str` items
  (`E0277`); corpus program 61.
* `out = out + "0".repeat(pad) + part` where `part` is a `String` and the left side an `&str` expression (`E0308`);
  corpus program 98.
