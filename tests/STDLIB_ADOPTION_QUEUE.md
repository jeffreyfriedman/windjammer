# Stdlib adoption queue (for the Windjammer std/runtime agent)

Ecosystem dogfooding needs these `std::*` surfaces. Each row has a **failing** `bug_std_*` repro under `tests/`. Prefer fixing std + runtime wiring over adding ecosystem workarounds.

Cross-link: ecosystem repo `docs/STDLIB_GRADUATION.md` (which packages graduate vs stay packages).

## P0 — unblock identity, encoding, crypto, time

| Need | Repro | Notes |
|---|---|---|
| `std::encoding.base64_encode_string` / `decode_string` | `bug_std_encoding_base64_string_api_test` | Runtime platform has fns; top-level encoding module must export |
| `std::random.range` → `random::int_range` | `bug_std_random_range_codegen_test` | UUID v4 |
| `std::crypto.sha1_bytes` | `bug_std_crypto_sha1_bytes_test` | UUID v5 |
| `std::crypto.sha256_hex` wiring | `bug_std_crypto_sha256_hex_wiring_test` | `wj-sha` should become thin |
| `std::time.utc_now` | `bug_std_time_utc_now_test` | UUID v1 |
| `DateTime.timestamp_millis` | `bug_std_time_timestamp_millis_test` | UUID v1 |
| `std::uuid` v4 | `bug_std_uuid_v4_module_test` | New std module (or crypto+random composition API) |

## P0 — HTTP / files everyone expects

| Need | Repro | Notes |
|---|---|---|
| `std::mime` constants + `from_extension` | `bug_std_mime_module_wiring_test` | Align std/mime.wj with runtime |
| `std::path` join / file_name | `bug_std_path_join_module_test` | Runtime `path` exists; WJ `std::path` missing/unwired |
| `std::jwt` sign/verify HS256 | `bug_std_jwt_hs256_wiring_test` | Runtime jwt.rs is real; std stub must codegen to it |

## P1 — config / data / db

| Need | Repro | Notes |
|---|---|---|
| `std::yaml` parse / stringify | `bug_std_yaml_module_test` | Missing module; ecosystem has pure subset only |
| `std::csv` idiomatic `Result<…, string>` | `bug_std_csv_parse_idiomatic_test` | Current std/csv.wj leaks Rust `csv.Error` |
| `std::db` connect + execute | `bug_std_db_execute_wiring_test` | Needed for in-WJ migrate apply |
| `std::time` parse human duration *or* RFC3339 roundtrip | `bug_std_time_rfc3339_roundtrip_wiring_test` | Deepen beyond stubs |

## Definition of done (per row)

1. Repro test turns green on tip (`cargo test --test all -- <filter>`).
2. Minimal idiomatic Windjammer program using the API compiles with local `wj` and runs.
3. Ecosystem package either thin-wraps std or is marked deprecated in favor of std.

## Run

```bash
cd windjammer
unset CARGO_TARGET_DIR
cargo test --release --test all -- bug_std_ -- --test-threads=1
```
