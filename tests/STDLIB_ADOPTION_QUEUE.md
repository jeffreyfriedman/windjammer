# Stdlib adoption queue (for the Windjammer std/runtime agent)

Ecosystem dogfooding needs these `std::*` surfaces. Each row has a **`bug_std_*` repro** under `tests/` that uses **`assert_stdlib_runtime_links`** (transpile + no `compile_error!` + runtime needle + **`cargo check`**). Substring-only gates false-green.

Cross-link: ecosystem repo `docs/STDLIB_GRADUATION.md`.

## Gate helper

```rust
test_utils::assert_stdlib_runtime_links(source, &["module::fn"]);
test_utils::assert_stdlib_runtime_links_any(source, &["time::utc_now", "time::now"]);
```

Definition of done per row:

1. Filtered repro is **green** on tip.
2. Idiomatic `use std::X` program compiles with local `wj` and runs.
3. Ecosystem `wj-*` package thin-wraps std or is deprecated.

## P0 — identity, encoding, crypto, time

| Need | Repro | Gate status (2026-08-24) | Fix hint |
|---|---|---|---|
| `std::encoding.base64_*_string` | `bug_std_encoding_base64_string_api_test` | ✅ | Runtime `base64_encode_string` / `decode_string` |
| `std::encoding.url_encode` / `url_decode` | `bug_std_encoding_url_encode_wiring_test` | ✅ | Runtime `url_encode` / `url_decode` (+ component aliases) |
| `std::random.range` → `int_range` | `bug_std_random_range_codegen_test` | ✅ | Done (alias + runtime) |
| `std::crypto.sha1_bytes` | `bug_std_crypto_sha1_bytes_test` | ✅ | Runtime SHA-1 + `std/crypto.wj` stub |
| `std::crypto.sha256_hex` | `bug_std_crypto_sha256_hex_wiring_test` | ✅ | Alias `sha256_hex` → hex digest |
| `std::time.utc_now` | `bug_std_time_utc_now_test` | ✅ | Runtime `DateTime` + `utc_now` |
| `DateTime.timestamp_millis` | `bug_std_time_timestamp_millis_test` | ✅ | Method on runtime `DateTime` |
| `std::uuid.v4` | `bug_std_uuid_v4_module_test` | ✅ | `std/uuid.wj` + runtime module |
| `std::uuid.v7` / `v7_from_timestamp` | `bug_std_uuid_v7_module_test` | ✅ | Runtime `uuid` `v7` + `std/uuid.wj`; thin-wrap `wj-uuid` |

## P0 — HTTP / files

| Need | Repro | Gate status | Fix hint |
|---|---|---|---|
| `std::mime` fn + constants | `bug_std_mime_module_wiring_test` | ✅ | Runtime consts + stdlib const scan → `.to_string()` |
| `std::path` join / file_name | `bug_std_path_join_module_test` | ✅ | String-oriented `join` / `file_name` |
| `std::jwt` HS256 | `bug_std_jwt_hs256_wiring_test` | ✅ | Done |

## P1 — config / data / db

| Need | Repro | Gate status | Fix hint |
|---|---|---|---|
| `std::yaml` | `bug_std_yaml_module_test` | ✅ | `std/yaml.wj` + `serde_yaml` → JSON text |
| `std::csv` idiomatic parse | `bug_std_csv_parse_idiomatic_test` | ✅ | Keep WJ `Result<_, string>` surface |
| `std::csv.write` owned rows (homonym `write`) | `bug_std_csv_write_owned_rows_auto_borrow_test` | ✅ | Package `pub fn write` → `csv.write(rows)` emits `&rows` |
| `std::db` connect + execute | `bug_std_db_execute_wiring_test` | ✅ | `execute` returns `i64` (WJ `int`) |
| `std::time` RFC3339 | `bug_std_time_rfc3339_roundtrip_wiring_test` | ✅ | `parse_rfc3339` + `DateTime::to_rfc3339` |
| `std::time.parse_duration_ms` / `format_duration_ms` | `bug_std_time_parse_duration_ms_wiring_test` | ✅ GREEN | Runtime helpers; `wj-duration` thin-wrap (P3.391) |
| `std::crypto.hash_password` / `verify_password` | `bug_std_crypto_bcrypt_password_wiring_test` | ✅ | Runtime bcrypt + `std/crypto.wj` → `windjammer_runtime::crypto` |
| `std::compress` gzip encode/decode | `bug_std_compress_gzip_wiring_test` | ✅ | Runtime `compress` + flate2; Base64 gzip string round-trip |
| `std::regex` is_match / find_all / escape | `bug_std_regex_module_wiring_test` | ✅ | Runtime `regex_mod` string APIs |

## P1 — concurrency (`wj-sync` graduation)

| Need | Repro | Gate status | Fix hint |
|---|---|---|---|
| `std::sync` unbounded channel + send/recv | `bug_std_sync_channel_shared_wiring_test` | ✅ GREEN | runtime `unbounded`/`send`/`recv`; `std/sync.wj` vocabulary; `wj-sync` thin-wraps `unbounded`/`bounded` |
| `std::sync` Shared get/add | `bug_std_sync_channel_shared_wiring_test` | ✅ GREEN | runtime `shared` / `shared_add` / `shared_get` (no WJ `SharedInt` stub — poisons package aliases) |
| `std::sync::atomic` AtomicI64 | `bug_std_sync_atomic_i64_wiring_test` | ✅ GREEN | Hot `Counter` for `wj-sync`; P3.370/P3.380 void i64 peers; Atomic* skip auto-Clone (bare-Counter path) |

## P1 — form / glob (beta week-one holes)

| Need | Repro | Gate status (2026-09-19) | Fix hint |
|---|---|---|---|
| `std::encoding.form_parse` / `form_stringify` | `bug_std_encoding_form_urlencoded_wiring_test` | ❌ RED | Runtime form-urlencoded; ordered `Vec<(string,string)>`; `+`/`%20` space; thin-wrap `wj-querystring` |
| `std::path.glob_match` | `bug_std_path_glob_match_wiring_test` | ❌ RED | Shell `*`/`?`/`**` segment rules; thin-wrap `wj-glob` |
| `strings.contains` owned needle → `&str` | `bug_std_strings_contains_owned_needle_test` | ❌ RED | Codegen demote owned/interpolated needle (E0308) |
| `std::config.resolve` / `merge` owned HashMap | `bug_std_config_module_test` (`std_config_resolve_must_wire`) | ❌ RED (3/4) | Codegen: owned formals must accept demoted `&mut HashMap` (or infer owned at call site) |

## Run

```bash
cd windjammer
unset CARGO_TARGET_DIR
cargo test --release --test all --features skip_fixtures -- bug_std_ -- --test-threads=1
```

Expected on tip: prior gates green; **form + glob + config.resolve** still RED until tip greens them.
