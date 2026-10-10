---
name: wj-cleanup
description: Reclaim disk and remove debris (rlibs, tmp dirs, stale worktrees, status docs) without losing work.
---
1. `df -h /`, `wj cache status`; if free disk is low run `wj cache prune`.
2. `git worktree list`; for each extra worktree check `git log HEAD..<wt>` and `git status`; archive dirty files to `~/src/_wj_archive/`, then `git worktree remove --force` and `git worktree prune`.
3. Delete untracked debris at repo roots (`*.rlib`, `tmp_*`, `*.zip`, `.agent-wip/`, `*_COMPLETE.md`, `SESSION_SUMMARY*`); find what generates it and fix the generator.
4. Commit with `chore:` and report bytes freed.
