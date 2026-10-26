# Ownership: a by-value store passed through adapter functions is inferred `&mut`

A `Store` whose methods take `self` by value and return a new store (`add(self, ..) -> Result<(Store, int), string>`,
`merge_from(self, ..) -> (Store, int)`), consumed by `fn apply(store: Store, ..)` in src/adapters/run.wj and
forwarded by `pub fn run_args(store: Store, ..)`, with `Store` defined in src/domain/store.wj.

`wj build src` emits `apply(store: &mut Store, ..)` and `run_args(store: &mut Store, ..)` (single-file and flat
two-file forms stay owned; the cross-module form does not), and callers pass `&Store`
(`E0308: types differ in mutability`). Found building the ecosystem `wj-todo-cli`.

Status: handed to the convergence stream. Ratchet: `ecosystem_findings_handoff_test::store_threaded_through_adapter_functions_stays_owned`,
listed in `.config/known-test-failures.txt`.
