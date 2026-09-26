# Tip agent handoff — form-urlencoded + path glob

**Status:** form + `config.resolve` GREEN (P3.463). Glob still RED. Ecosystem can thin-wrap `encoding.form_*`; stay on `wj-glob` until that gate greens.

## Ask

1. ~~Green `bug_std_encoding_form_urlencoded_wiring_test`~~ — done P3.463.
2. Green `bug_std_path_glob_match_wiring_test` — `std::path.glob_match`.

Also still open:

- `bug_std_strings_contains_owned_needle_test` — `strings.contains(hay, "\"${key}\"")` must demote owned needle to `&str` (E0308)

## Form API contract (`std::encoding`)

`application/x-www-form-urlencoded` is a platform codec (same family as `url_encode`). Ordered pairs preserve repeated keys.

| Fn | Role |
|---|---|
| `form_parse(text) -> Vec<(string, string)>` | Parse `a=1&b=2` (optional leading `?`); decode `+` and `%HH` |
| `form_stringify(pairs) -> string` | Serialize; encode spaces as `+`, reserved as `%HH` |

Runtime needles: `encoding::form_parse`, `encoding::form_stringify`.

Reference behavior: ecosystem `packages/wj-querystring` (pure WJ over existing `url_encode`/`url_decode`).

## Glob API contract (`std::path`)

| Fn | Role |
|---|---|
| `glob_match(pattern, path) -> bool` | Shell-style `*` / `?` (one segment) and `**` (zero+ segments) |

Runtime needle: `path::glob_match`.

Reference behavior: ecosystem `packages/wj-glob`.

## After tip GREEN

1. Thin-wrap `wj-querystring` `parse`/`stringify` over `std::encoding` form_* (keep `get`/`append` sugar in package).
2. Thin-wrap `wj-glob` `is_match` over `path.glob_match`.
3. Mark rows green in `STDLIB_ADOPTION_QUEUE.md` + `docs/STDLIB_GRADUATION.md`.

## Gate copies

Mirrored under ecosystem `.tmp-std-form/` if tip tree is busy/`uchg`.
