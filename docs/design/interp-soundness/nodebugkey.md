---
status: current
last_verified: 5457283f4156
---
# Soundness: path-free AST Debug text in the resolved engine

Finding: none of the Debug-formatted sites under src/interpreter/ is a KEY or an identity. Each formats an AST node
and splits the text into words to over-approximate "the locals this closure/island may name" (`closure::create`,
`closure::compile`, `Island::new/with_copies` from compile.rs stmt_island, expr.rs expr_arg/expr_eval, matching.rs,
method.rs) or to test whether a captured name occurs in a closure body (`builtins/thread.rs move_captures`:
`name: "x"`). Nothing is stored in a map under the text, nothing is persisted, hot reload does not see it (the text is
built per compile and dropped; a reloaded body is compiled afresh from its new AST). So the task's uniqueness /
aliasing / cross-file / persistence questions do not arise: there is no key to alias, and a stable id would add a
lookup rather than remove one.

What the path did: `Location { file: PathBuf, line, column }` is in the AST Debug; printing it cost O(path length)
per node (one PathBuf Debug call per node) plus the word-splitting and HashSet inserts of its components.

Change: `Location`'s Debug (src/source_map.rs) prints `file: ""` when the formatter carries the elision precision
(`ELIDE_PATH_PRECISION`, set only by `source_map::elided_debug`); otherwise it is byte-identical to the derived one (unit
test). The sites call it through `island::ast_text` (thread.rs directly).

Why sound: the word set loses only the words of path components (and the file string's escapes). A path is not
code, so no local can be named by it; losing those words can only shrink an OVER-approximation by names that were never
used (e.g. a local called `home` was previously boxed because `/home/..` appeared in the path: a pure slowdown, never
a semantic change, and cwd-dependent semantics-visible boxing was impossible by the walker/resolved parity gate).
`line`/`column`/`Location`/`file` words remain, so the set differs from the old only by path words. The
"Closure {" test is unaffected (a path never contains it).

Mutations (tests/interpreter_closure_keys_test.rs, results on pc-results): M1 ast_text -> "" (captures lost):
the environment test fails; M2 elision off: the Debug test fails (and the per-path-character cost returns).
Left alone: kmap.rs:250 seeded_hash hashes the Debug text of a map-key Value (data, not AST; keys are not paths); changing a map-order hash would change iteration order, out of scope.
