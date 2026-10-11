# Claude Code skill pack

Copy this directory to `.claude/skills/windjammer/` in your project (or `~/.claude/skills/windjammer/`):

```bash
mkdir -p .claude/skills/windjammer
cp SKILL.md ../WINDJAMMER_FOR_AGENTS.md .claude/skills/windjammer/
```

Optionally register the MCP server so the agent can call `check_code`, `run_code`, `test_code`, `format_code`,
`explain_error` and `docs_search` directly:

```bash
claude mcp add windjammer -- windjammer-mcp
```

`SKILL.md` is checked by CI (`tests/agent_docs_test.rs`) against the real CLI surface.
