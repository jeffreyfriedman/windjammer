# LedgerKit `wj` pin (api-check)

LedgerKit `make api-check` is **GREEN on `next/v0.50.0` tip** after P3.336 (2026-09-17). Use `target/release/wj` built from tip; no binary pin.

| Field | Value |
|-------|--------|
| Last GREEN commit (api-check on LedgerKit platform) | tip (`next/v0.50.0`) |
| Regression window | `99503c93` (P3.323) … P3.336 fix |
| Gate | `int_mod_literal_zero_compare_must_not_split_i64_i32` |

```bash
cd ~/src/wj/windjammer
cargo build --release -p windjammer --bin wj
# financial-management-platform: make api-check
```
