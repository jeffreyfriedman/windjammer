# Specialized Subagents for Windjammer Development

**Date:** 2026-03-14  
**Location:** `~/.cursor/agents/` (installed globally)

## Overview

Created 6 specialized subagents based on interaction patterns and `.cursor/rules/`:

1. **tdd-implementer** - TDD specialist (tests first, no stubs)
2. **rust-leakage-auditor** - Audits for Rust-specific patterns
3. **compiler-bug-fixer** - Fixes compiler bugs with TDD
4. **visual-verifier** - Visual quality verification with screenshots
5. **dogfooding-validator** - Validates compiler on real game code
6. **performance-profiler** - Performance optimization with profiling

All subagents use `model: inherit` to match parent agent's model.

## Usage

Subagents are automatically available. Use them explicitly:

```bash
# In Cursor chat
> /tdd-implementer implement feature X with tests first
> /rust-leakage-auditor audit rendering/*.wj for Rust leakage
> /compiler-bug-fixer fix ownership inference bug
> /visual-verifier verify game rendering with screenshots
> /dogfooding-validator validate compiler on breach-protocol
> /performance-profiler profile voxel rendering performance
```

Or let Agent delegate automatically based on task.

## See Also

- Full documentation in each `.md` file: `~/.cursor/agents/*.md`
- Session summary: `PARALLEL_TDD_SESSION_SUMMARY_2026_03_14.md`
- Interaction patterns: `.cursor/rules/*.mdc`
