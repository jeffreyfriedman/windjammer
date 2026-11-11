Neutral minimal programs for the failure classes seen in pilot 6 (`docs/drift/pilot6-findings.md`). Each program is accepted by `wj check` and run correctly by
`wj run --interpret`; the compiled build (`wj run --compiled`) rejects it or, for the tooling entry, prints another program's output. Checked on the compiler of the
pilot (`wj 0.50.0`, commit 82d5ac0579cd).
