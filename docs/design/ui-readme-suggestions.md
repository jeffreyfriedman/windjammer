---
status: current
last_verified: 203e43d114c9
---
# windjammer-ui README: suggestions (not applied)

The UI vision is in flux, so this repository does not edit `windjammer-ui`. Observations from its current 59-line
README and what to do when the owner is ready.

## What is good
- Leads with "Status: experimental" and a platform-status table with stubs named. Do not lose this.
- Short, with a code sample near the top.

## Suggestions
1. Positioning line first: what a developer builds with it, in one sentence, not what it contains ("components and a
   reactive runtime" says what, not why). Decide the vision first; then the line follows.
2. Replace the sample with one that shows the differentiator (Windjammer source producing a reactive page) plus the
   command that serves it, and make the sample a test in that repository (compile plus an assertion on rendered HTML),
   the same way this repository's README samples are tested.
3. A "run it in 60 seconds" block: clone, one build command, open a URL. If that needs `wasm-pack` or a pinned compiler
   (`WJ_PIN`), say so in the block.
4. Replace "about 120 component sources" with a generated count (script plus CI check) or drop the number.
5. Add an agent section: link to this repository's `llms.txt` and `docs/agent/WINDJAMMER_FOR_AGENTS.md`, state which
   component API an agent should treat as stable, and ship an `AGENTS.md` (it has one; link it).
6. Screenshot or short recording of one real example. Do not add one until the visual design settles.
7. Badges: CI and license only, and only if CI exists for that repository.
8. Keep "Known limitations" but link each to an issue or doc, so it reads as a roadmap rather than an apology.
9. Never name private downstream projects (the "ledger-specific components" remark should become "domain-specific
   components scheduled to move out" in public text).
