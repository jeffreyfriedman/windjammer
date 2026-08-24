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
| `std::random.range` → `int_range` | `bug_std_random_range_codegen_test` | ✅ | Done (alias + runtime) |
| `std::crypto.sha1_bytes` | `bug_std_crypto_sha1_bytes_test` | ✅ | Runtime SHA-1 + `std/crypto.wj` stub |
| `std::crypto.sha256_hex` | `bug_std_crypto_sha256_hex_wiring_test` | ✅ | Alias `sha256_hex` → hex digest |
| `std::time.utc_now` | `bug_std_time_utc_now_test` | ✅ | Runtime `DateTime` + `utc_now` |
| `DateTime.timestamp_millis` | `bug_std_time_timestamp_millis_test` | ✅ | Method on runtime `DateTime` |
| `std::uuid.v4` | `bug_std_uuid_v4_module_test` | ✅ | `std/uuid.wj` + runtime module |

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
| `std::db` connect + execute | `bug_std_db_execute_wiring_test` | ✅ | `execute` returns `i64` (WJ `int`) |
| `std::time` RFC3339 | `bug_std_time_rfc3339_roundtrip_wiring_test` | ✅ | `parse_rfc3339` + `DateTime::to_rfc3339` |

## Run

```bash
cd windjammer
unset CARGO_TARGET_DIR
cargo test --release --test all --features skip_fixtures -- bug_std_ -- --test-threads=1
```

Expected on tip: **24/24** `bug_std_*` green.
