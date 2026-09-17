# LedgerKit `wj` pin (api-check)

When `next/v0.50.0` tip source fails LedgerKit `make api-check` (~289 i64/i32 width errors), use a **known-good compiler binary** without moving the branch tip:

| Field | Value |
|-------|--------|
| Last GREEN commit (api-check on LedgerKit platform) | `27255538` |
| First RED after that | `99503c93` (P3.323) |
| Branch tip (source stays here) | `next/v0.50.0` @ `a6bb24e6`+ |

```bash
git worktree add --detach /tmp/wj-ledgerkit-green 27255538
cd /tmp/wj-ledgerkit-green
CARGO_TARGET_DIR=/tmp/wj-ledgerkit-green-target cargo build --release -p windjammer --bin wj
cp /tmp/wj-ledgerkit-green-target/release/wj ~/src/wj/windjammer/target/release/wj
```

Track fix progress with gate `int_mod_literal_zero_compare_must_not_split_i64_i32` (P3.336).
