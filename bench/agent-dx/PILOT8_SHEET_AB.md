# Pilot 8 sheet experiment: A/B design (ready, not run)

Nothing here has been run; no paid call was made. Launch is the owner's decision (`python3 -m agentdx pilot8 --dry-run`
validates the matrix offline once the driver entry is added; the arms and packs below are already in `agentdx/arms.py`
and `packs/pilot8/`).

## Questions

1. **Idioms (turns).** Does a sheet that carries the six idioms agents looked up with `wj docs` before writing
   (split+collect, exit status, trait objects, recursive enums, owned-string spelling, mutating through `self`) remove the
   pre-write `wj docs` turns? Pilot 7: 1.55 docs turns before the first write per U7 trial (0.60 for V3), 2.2 docs calls per trial.
2. **Opening (thinking).** Does a shorter, sharper opening cut first-draft thinking? Pilot 7: thinking is 14.3k tokens per
   Windjammer trial against 7.3k for Rust, 2.4x already in trials of at most 3 turns, and it is 44% of the USD gap
   (0.0072 of 0.0164 per trial). Halving it is worth about 11% of the cost per trial.
3. **Verify signal.** With `wj run --verify` exiting 0 whenever the runs agree (this branch), do V arm trials stop
   re-running a correct program that exits 1 on purpose (pilot 7: 51% of V3 failing-run samples)?

## Arms (haiku; tasks t302-t309 plus t103 and t104; same task set and oracles as pilot 7)

| Arm | Loop | Sheet (`packs/pilot8/`) | Differs from |
|---|---|---|---|
| U7 | compiled-first | `../pilot7/sheet_u.md` (control, rerun the same day) | - |
| U8 | compiled-first | `sheet_u8.md` | U7 by the idiom block only |
| U8s | compiled-first | `sheet_u8s.md` | U8 by the opening only |
| V4 | `verify-ok` | `sheet_v4.md` (idioms, old opening, loop bullet that names the OK line) | V3 by idioms and the loop text |

Opening texts (the only difference between U8 and U8s):

- U8 (control opening, pilot 7): "Windjammer (.wj) is a Rust-family language with inferred ownership. It is not in your training
  data: do not guess from Rust or Python."
- U8s (sharp): "Windjammer (.wj): Rust-like, ownership inferred. Follow this sheet exactly; where it is silent, `wj docs`
  answers in one call. Write the whole program once, then run it."

The sharp opening drops "not in your training data" (which invites doubt and re-derivation) and states the decision
procedure instead: write once, check once. It is a hypothesis; the experiment can reject it.

## Metrics (all recorded by the harness already)

Primary: first-write thinking tokens (`modelUsage.thinkingTokens`), assistant turns before the first Write, `wj docs`
calls, turns to the final state, USD per trial. Secondary: pass rate (oracle), failing-run share that is program output,
denials (must be 0: the harness fix is a precondition, `DENIAL_MAX_SHARE` guard at 0.25).

## Size and cost

- 3 trials per cell, 10 tasks, 4 arms = 120 haiku trials.
- Expected: 0.0165 USD per trial (pilot 7 U7 0.0164, V3 0.0166) = about 2.0 USD. The idiom block adds 150-200 input
  tokens per turn (cache read, about 0.00002 USD per turn): negligible. Per-trial cap 0.10 (as pilot 7) gives a hard
  ceiling of 12 USD, the ratchet stops the matrix at 25% denials or 3 USD spent beyond the expectation.
- Power: pilot-7 haiku thinking SD is about 6k tokens, turns SD about 5. With 30 trials per arm the standard error is
  1.1k tokens (detects a 3k = 20% thinking difference) and 0.9 turns (detects -1.5 turns, the predicted idiom effect;
  -1.0 is at the edge). Paired by task for the opening comparison (same tasks in all arms).
- Decision rules, fixed before the run: ship the idiom block if U8 shows at least -0.8 docs turns before the first write
  and no pass-rate loss; ship the opening if U8s thinking is at least 15% below U8 and the pass rate is not lower by
  more than 1 trial of 30. Otherwise keep the pilot-7 text. The sheet budget (`SHEET_BUDGET_BYTES`) is raised only
  with a shipped idiom block, and by the measured amount.

## Preconditions

Harness permission denials fixed (separate change); this branch's `--verify` exit rule; the iterator-as-list and
diagnostic changes of the other streams are in the binary under test, and the binary commit is pinned in the matrix
config so the arms differ only in the sheet.
