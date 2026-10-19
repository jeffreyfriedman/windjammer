# The Rust-leakage lints are compile errors in `wj build` but absent in `wj run --interpret`

A program developed against the interpreter is rejected by the compiled path for style alone. Hard errors (exit non-zero) seen by agent-dx:

| Lint | Triggered by | Note |
|------|--------------|------|
| W0002 | `.unwrap()` | |
| W0003 | any `.iter()` call (also `.iter().map(..)`) | `for x in v` is required |
| W0004 | explicit `&` / `&mut` in a call argument | `io::stdin().read_line(&mut line)` is therefore unwritable |
| W0006 | `"literal".to_string()` | |
| W0010 | the type name `String` (also inside `collect::<String>()`) | use `string` |
| W0001 | explicit `&mut self` | but accepted inside `impl Trait for T` |
| (warning) | `.as_str()` | was a hard error; now a warning in every mode, removed by `wj fix` |

Conversely the interpreter needs `.as_str()` before `s[a..b]` on `String` values and a collected `Vec` for `env::args()` indexing, while the
compiled path forbids `.as_str()` and cannot `.collect()` `env::args()` (already a `Vec`: E0599). Programs that work in both modes need
loop-based helpers for these; see `bench/agent-dx/solutions/wj/`. `wj lint`/`wj check` run on the interpreter-targeted file would surface
the compiled-mode errors earlier (suggestion: make `wj run --interpret` print the same W-lints as warnings).
