# Agent Instructions

**Read the [windjammer-engineering skill](../.cursor/skills/windjammer-engineering/SKILL.md) before starting work.**

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Branch Strategy (Parallel Development)

This repository has two active development tracks:

| Branch | Purpose | Owner | Worktree |
|--------|---------|-------|----------|
| `fix/post-0.48.0-bugs` (→ `main`) | Stable compiler: bugfixes, dogfooding | Game/bugfix agent | `windjammer/` |
| `next/safety-typed-ir` | Next-gen architecture: Safety-Typed IR | Architecture agent | `windjammer-next/` |

### Rules for `next/safety-typed-ir` (this branch)

- This branch introduces `src/ir/` — the Safety-Typed IR layer
- Periodically sync from `main` to pick up bugfixes: `git merge main`
- All 4180+ existing tests must pass at every commit
- Cutover to `main` as v0.50.0 only after full validation (tests + game build + playtest)

### Rules for `main` / `fix/*` (other agents)

- Do NOT modify `src/ir/` — that module belongs to the architecture branch
- All other compiler work (bugfixes, ownership inference, codegen) continues normally
- Tag releases as v0.48.x or v0.49.x
- The game projects (`breach-protocol`, `windjammer-ui`) use the stable compiler from this track

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

Use 'bd' for task tracking

## Compiler tests

Add `tests/your_feature_test.rs` — `build.rs` picks it up automatically. No edits to `all.rs` or `tests/lib.rs`.

**Canonical full suite:**

```bash
cargo test --release --test all
```

See `tests/README.md` for suite features and filtering.

Compiler-only changes: tests + `cargo test --release --test all` in `windjammer/` are sufficient.

When fixes enable or change player-visible behavior in **Breach Protocol** or the engine render path, agents must validate through the game — not only unit tests:

1. Build the game: `wj game build --release` in `../breach-protocol/`
2. Headless playthrough with screenshots (see `../breach-protocol/AGENTS.md`)
3. Dual-persona jury per `../.cursor/rules/dual-persona-jury-evaluation.mdc`

Do not claim rendering or gameplay fixes are done without screenshot evidence and three-tier verdict.
