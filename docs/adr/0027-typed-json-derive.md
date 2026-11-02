---
status: proposed
last_verified: 4b18e8c2e
---
# ADR-0027: Typed JSON for structs and enums (one data model, two engines)

- **Status:** Proposed (minimum vertical slice implemented)
- **Date:** 2026-11-02
- **Related:** [ADR-0020](0020-interpreter-execution-engine.md), [ADR-0021](0021-effects-and-capabilities.md) (effects),
  [ADR-0022](0022-interpret-by-default.md) (interpret by default, compiled for release),
  [ADR-0014](0014-surface-syntax-rust-isms.md) (no Rust attribute leakage), `std/json.wj`,
  `crates/windjammer-runtime/src/json.rs`, `src/interpreter/builtins/json_codec.rs`,
  `src/interpreter/builtins/std_data.rs`, `src/codegen/rust/trait_derivation.rs`,
  `src/codegen/rust/item_generation.rs`, `src/analyzer/forbidden_patterns.rs`, `tests/json_conformance/`

## Context

A struct or enum should convert to and from JSON with no hand-written glue, then TOML and YAML through the same
machinery. Agents handle JSON constantly; the program shape "read a request, decode it into a type, act, encode a
reply" is the everyday 80%.

### What exists today (survey, `next/v0.50.0` at `4b18e8c2e`)

| Piece | Where | State |
|---|---|---|
| Typed surface | `std/json.wj` | `json.to_string(v)`, `json.to_string_pretty(v)`, `json.parse_string::<T>(text)` (target type from turbofish, `let x: Result<T, string>`, or `?`/`match` context). Errors are `string`. |
| Dynamic surface | `std/json.wj` | `json.parse`, `json.Value`, `get`/`as_*`/`keys`/`set`, builders. Unchanged by this ADR. |
| Compiled derive | `codegen/rust/trait_derivation.rs` | **Implicit.** When the project uses `std::json` (`serde_available`/`needs_serde_imports`) every struct whose fields are serde-derivable gets `#[derive(Serialize, Deserialize)]`. `@derive(Serialize)` is rejected by `analyzer/forbidden_patterns.rs` ("auto-derived project-wide"). |
| Compiled enums | `codegen/rust/item_generation.rs::generate_enum` | **No** serde derive. A struct with an enum field derives `Serialize` anyway (`type_is_serde_derivable` treats the enum as local and derivable), so rustc rejects the program. |
| Compiled runtime | `windjammer-runtime/src/json.rs` | `serde_json::from_str(..).map_err(to_string)`: message has line/column, no field path. |
| Interpreter | `interpreter/builtins/std_data.rs` (`to_json`/`from_json`) | Reflection over the declared type, hand-imitating serde's encoding. Messages are imitations (`missing field \`qty\``) without line/column or path; unknown fields are ignored; integer overflow and float-into-int texts differ from serde's. |
| Field decorators | `item_generation.rs` | Any unknown field decorator is emitted verbatim as a Rust attribute (`@serde(rename = "x")` would become `#[serde(rename = x)]` on the compiled path and be ignored by the interpreter). |
| Consumers | `wj-json-doc`, `wj-json-util` (windjammer-ecosystem) | Value-level (ordered documents, dotted paths, merge). They do not need typed decoding; programs that hand-walk `json.Value` today are the migration target. |
| Effects | `src/ir/effect_facts` | `std::json` is not in the sink table: encoding and decoding are pure. |

So the gap is not "no derive". It is that the two engines implement two different contracts, neither of which is
written down, and the compiled path cannot encode enums.

## Options

### A. Explicit decorator `@derive(Json)`, code generated at the IR level

The compiler synthesizes `to_json`/`from_json` Windjammer functions per type, which every backend lowers.

- For: one implementation for every backend (Rust, JS, Go, interpreter) and no serde dependency.
- Against: re-implements a JSON reader/writer, number formatting and error reporting in generated code, for every
  backend, and has to match serde's output byte for byte on the release path anyway. Ceremony on every type, against
  the existing (shipped) policy that the derive is implicit. Large generated code per type.

### B. Compiler-synthesized std trait impl (implicit derive), interpreter by reflection: status quo

Keep what exists and patch divergences one by one.

- For: no surface change.
- Against: two independent codecs drift; every divergence (unknown fields, overflow text, path) is found by users.

### C. One data model: serde's, ridden by both engines (chosen)

The derive stays implicit (any struct or enum whose fields are encodable, when the program uses `std::json`). The
contract is serde's data model with Windjammer's policy on top. The compiled path uses `serde` derive; the
interpreter implements `serde::Serialize` and `serde::de::DeserializeSeed` driven by the declared type, and both run
through the same `serde_json` serializer/deserializer and the same `serde_path_to_error` adapter. Output bytes, error
texts, line/column and paths then agree by construction; a shared conformance corpus proves it per release.

- For: parity by construction rather than by imitation; TOML and YAML come from the same seed with
  `toml`/`serde_yaml` deserializers; no surface change for existing programs.
- Against: the interpreter depends on serde's trait shapes (already a dependency); non-Rust backends need their own
  codec later (ladder rung 5), checked against the same corpus.

## Decision

Option C, with this contract:

| Topic | Decision |
|---|---|
| Opt-in | Implicit, as today: no decorator. `@derive(Serialize)`/`@derive(Deserialize)` stay forbidden (they are Rust leakage). |
| Surface | `json.to_string(v)`, `json.to_string_pretty(v)`, `json.parse_string::<T>(text) -> Result<T, string>`. No new names. |
| Struct encoding | JSON object, fields in declaration order. Tuple structs: one field is the inner value, more is an array. |
| Field renames | `@json(rename = "userName")` on a field (in the slice). Not `@serde(...)`: the surface is Windjammer's, the backend chooses the implementation. |
| `Option<T>` | Missing or `null` decodes to `None`; `None` encodes as `null` (`@json(skip_if_none)` at rung 2). |
| Defaults | A missing non-`Option` field is an error. `@json(default)` (the type's `Default`) at rung 2. |
| Unknown fields | **Denied by default** on decode (`unknown field \`x\`, expected one of ...`). Opt out per struct with `@json(allow_unknown)` (in the slice, so denial never strands a program). Rationale: a misspelt field silently becoming `None` is the most expensive JSON bug for agents; a loud error with the expected set is cheap to fix. |
| Enums | Externally tagged (serde's default): unit `"V"`, newtype `{"V": x}`, tuple `{"V": [..]}`, struct `{"V": {..}}`. Internally/adjacently tagged via `@json(tag = "type")` / `@json(tag, content)` at rung 2. |
| Nested, `Vec`, maps | Nested structs and enums recurse; `Vec<T>` is an array; `HashMap<string, V>`/`BTreeMap` are objects (integer keys are written as strings, as serde_json does). |
| Integers | `int` is `i64`. Every declared width is range-checked: `300` into `u8` is `invalid value: integer \`300\`, expected u8`; a number beyond `i64` or with a fraction into an integer field is an error, never truncated. Integers decode into float fields. |
| Floats | Shortest round-trip text (`1.0`, `1e20`); NaN and infinities encode as `null`. |
| Errors | `string` with a fixed grammar: `<path>: <message> at line <L> column <C>`, where `<path>` is a JSONPath (`$`, `$.home.city`, `$.tags[2]`, `$["odd key"]`) and `<message>` is serde's text. A missing field reports the object that lacks it (`$.home: missing field \`city\``); an unknown field reports itself (`$.extra: unknown field \`extra\`, expected \`city\` or \`zip\``); a type error reports the value (`$.tags[1]: invalid type: integer \`2\`, expected a string`). Syntax errors and trailing characters use the same grammar. A typed `json.Error { path, line, column, message }` is rung 2. |
| Engines | Interpreted and compiled must produce identical bytes and identical error strings. The oracle is one corpus (`tests/json_conformance/*.wj`, `// expect:` lines) run through both engines; the expected encodings are also checked as wire format against Python's `json` module (golden files), which is not the behavioural oracle. |
| Effects | Pure: no capability required, no effect reported; a `@pure` function may encode and decode (`typed_json_is_pure` in `tests/json_conformance_test.rs`). |

### Compiled path changes

- `#[serde(deny_unknown_fields)]` on every struct that derives `Deserialize`.
- Enums derive `Serialize, Deserialize` under the same condition as structs (fixes the enum-field build break).
- `windjammer-runtime::json::parse_string` decodes through `serde_path_to_error` and formats the path as JSONPath.

### Interpreter path changes

`src/interpreter/builtins/json_codec.rs`: `Encode<'a>(&'a Value)` implements `serde::Serialize`;
`Decode { ty, defs }` implements `DeserializeSeed`, calling the same `deserialize_*` methods and the same
`de::Error` constructors (`missing_field`, `unknown_field`, `invalid_type`) that serde derive calls, and delegating
scalars to the real `i64::deserialize`, `u8::deserialize`, `String::deserialize`, ... implementations.

### Version skew is a parity hazard

Generated programs resolve `serde`/`serde_json` from the registry when they are first built, while the interpreter
uses the versions in this repository's `Cargo.lock`. The first corpus run caught exactly that: serde 1.0.228 prints
an unexpected float as `1e20`, 1.0.229 as `1e+20`. The slice aligns the lock with what generated programs resolve
today (serde 1.0.229, serde_json 1.0.154). The durable fix (rung 2) is to pin generated programs to the
interpreter's versions (`=` requirements in the generated `Cargo.toml`, taken from this lock at build time), so the
corpus stays green when serde releases.

## Ambition ladder

| Rung | Outcome | First experiment / gate | Kill criterion |
|---|---|---|---|
| 1 (this slice) | Structs of scalars, `string`, `Option`, `Vec`, nested structs, externally tagged enums, `@json(rename)` and `@json(allow_unknown)`: encode and decode, identical bytes and errors in both engines; unknown fields denied; JSONPath errors | `tests/json_conformance` run interpreted and compiled | — |
| 2 | `@json(default, skip_if_none, tag, content, rename_all)`; typed `json.Error`; generated programs pinned to the interpreter's serde versions | corpus cases per attribute, both engines | attribute needs per-engine semantics that cannot share the seed |
| 3 | TOML and YAML from the same seed: `config.parse_string::<T>`, `yaml.parse_string::<T>` | the corpus re-run with a TOML/YAML rendering of every case | serde's TOML/YAML data model differs per engine (then document, do not paper over) |
| 4 | Schemas from types: a schema subcommand emits JSON Schema for a type; MCP tool input/output schemas derived from Windjammer structs, so an agent's tool contract is the type | schema of every corpus type validated against its own accepted/rejected cases | schema and decoder disagree on any case |
| 5 | JS and Go backends encode/decode with generated codecs (no serde), same corpus | corpus through the JS backend | — |
| 6 | Performance: interpreted decode within 3x of compiled on a 10 MB document; streaming decode of arrays | `bench/` case under load-aware benchmarking | interpreter overhead dominated by `Value` allocation (then revisit with arenas, ADR-0025) |

## Consequences

- Programs that relied on unknown fields being ignored now fail to decode until they add `@json(allow_unknown)`
  or declare the field. This is the intended trade; it is called out in the changelog.
- Error strings change shape (path prefix, line/column). Programs that matched exact error text need updating.
- The interpreter's JSON codec no longer imitates serde: it calls it.
