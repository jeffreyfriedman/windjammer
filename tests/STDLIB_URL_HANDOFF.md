# Tip agent handoff — URL + int/usize loop unify

**Status:** Ecosystem `wj-url` **15/15 tip green** — authority scan isolates host/port on a substring (no `j < auth_end` cross-bound). Tip still owns greening index unify + `std::url`.

## Ask

1. Green `bug_int_loop_assign_end_bound_must_unify_test` — loop counter assigned from another index/`len`-derived bound must share one integer width (no `i32` vs `usize` E0308).
2. Green `bug_std_url_parse_wiring_test` — `std::url.parse` / `format` / `join`.

## URL API contract (`std::url`)

| Fn | Role |
|---|---|
| `parse(text) -> Result<Url, string>` | Absolute `scheme://host[:port][/path][?query][#fragment]` |
| `format(u) -> string` (alias `format_url`) | Serialize |
| `join(base, rel) -> Result<string, string>` | Resolve relative against base |

`Url` fields: `scheme`, `host`, `port`, `path`, `query`, `fragment` (all `string`).

Reference: ecosystem `packages/wj-url`. Query helpers stay package sugar (prefer `encoding.form_*` / `wj-querystring` once form greens).

## After tip GREEN

Thin-wrap `wj-url` parse/format/join over `std::url`; keep query_* sugar.

## Gate copies

`.tmp-std-url/` in ecosystem if tip tree is busy.
