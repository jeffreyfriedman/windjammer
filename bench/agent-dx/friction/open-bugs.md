# Open compiler bugs in the friction corpus

Every event that `replay.wj` finds still failing on the current compiler (`open-compiler-bug`: `wj check` accepts the
program, the compiled build rejects it) has a line here with the stream that owns the fix. `replay.wj` and
`tests/agent_cost_ratchet_test.rs` fail when an open event is missing; delete a line when the replay reports the event
`fixed`. Owner streams: `closure-values` (codegen of closures and blocks used as values), `ownership` (ownership and
String inference), `adapters` (iterator adapters and collect), `widths` (integer width solver), `traits`, `std-surface`.
Programs: `events/<id>/main.wj`; minimal repros of the pilot-6 classes are in `docs/drift/pilot6-repros/`.

Status 2026-11-13 (wt-width-13 replay, events_open = 3): the three events below are open and none is a width event: `p3-t006-B-0-c00` (std-surface), `p3-t034-T-0-c00` (traits), `p3-t050-F-0-c00` (std-surface; the WJ0015 diagnostic of std-surface5 closes it once merged).

| Event | Owner stream | Class | First error on the current compiler |
|---|---|---|---|
| `p3-t006-B-0-c00` | std-surface | char predicate argument | WJ0017: trait bound not satisfied: expected an `Fn(char)` closure, found `char` |
| `p3-t034-T-0-c00` | traits | trait used as a type | WJ0034: a trait cannot be used as a plain type |
| `p3-t050-F-0-c00` | std-surface | `len` on a split iterator | WJ0015: no method `len` found for `std::str::Split<'a, P>` |
