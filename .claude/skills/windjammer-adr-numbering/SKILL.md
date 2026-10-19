---
name: windjammer-adr-numbering
description: Use when adding or renumbering a Windjammer ADR.
category: git-workflow
---

# Windjammer ADR numbering

Worktrees number ADRs independently, so a merge can bring a number that is already taken. Choose the number against the target branch, not the worktree.

- Run `ls docs/adr` on the target branch and take the next free number.
- On a collision, renumber the incoming file with `git mv`. Then update its title, its `Related:` line and every reference to it.
- Update link targets as well as labels: `[ADR-0021](0021-hot-reload.md)` needs both the label and the path changed. Search with `grep -rn 'ADR-00NN\|NNNN-slug' docs src tests README.md`. Source comments in `src/` count.
- Update `docs/adr/README.md` if it indexes the ADRs.
- Record the current numbering in session memory so the next agent does not reuse a number.