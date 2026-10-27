"""Parse `claude -p --output-format stream-json --verbose` output into the final result plus a per-turn token log."""
import json


def _text_len(content):
    if isinstance(content, str):
        return len(content)
    if isinstance(content, list):
        return sum(_text_len(c.get("text", "") if isinstance(c, dict) else c) for c in content)
    return 0


def parse(text):
    """-> (result_json | None, turn_log, tools_exposed). Assistant events sharing a message id are one turn."""
    result, tools, turns, order = None, [], {}, []
    pending = None   # turn id whose tool results are being collected
    for ln in text.splitlines():
        ln = ln.strip()
        if not ln.startswith("{"):
            continue
        try:
            ev = json.loads(ln)
        except ValueError:
            continue
        t = ev.get("type")
        if t == "system" and ev.get("subtype") == "init":
            tools = list(ev.get("tools") or [])
        elif t == "result":
            result = ev
        elif t == "assistant":
            m = ev.get("message") or {}
            mid = m.get("id") or f"anon{len(order)}"
            if mid not in turns:
                turns[mid] = {"calls": [], "result_chars": 0}
                order.append(mid)
            rec = turns[mid]
            u = m.get("usage") or {}
            if u:
                rec["usage"] = u
            for c in m.get("content") or []:
                if isinstance(c, dict) and c.get("type") == "tool_use":
                    rec["calls"].append(c.get("name", "?"))
            pending = mid
        elif t == "user" and pending:
            for c in (ev.get("message") or {}).get("content") or []:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    turns[pending]["result_chars"] += _text_len(c.get("content"))
    log = []
    for i, mid in enumerate(order, 1):
        u = turns[mid].get("usage") or {}
        fresh, cw, cr = u.get("input_tokens", 0) or 0, u.get("cache_creation_input_tokens", 0) or 0, u.get("cache_read_input_tokens", 0) or 0
        log.append({"turn": i, "input_total": fresh + cw + cr, "fresh": fresh, "cache_write": cw, "cache_read": cr,
                    "output": u.get("output_tokens", 0) or 0, "calls": turns[mid]["calls"], "result_chars": turns[mid]["result_chars"]})
    return result, log, tools


def summarise(log, tools):
    """Per-trial attribution inputs: first-turn context, tool-result tokens (chars/3.5), call counts by kind."""
    calls = [c for t in log for c in t["calls"]]
    mcp = {}
    for c in calls:
        if c.startswith("mcp__"):
            mcp[c] = mcp.get(c, 0) + 1
    return {"first_turn_input": log[0]["input_total"] if log else None,
            "tool_result_tokens_est": round(sum(t["result_chars"] for t in log) / 3.5),
            "mcp_calls": mcp, "n_tool_calls": len(calls),
            "tools_exposed": sorted(tools)}
