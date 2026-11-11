"""Cost decomposition of one trial (pilot 7, ADR-0037): where the dollars go.

Inputs are what a trial record holds: the final usage split reported by Claude Code (fresh input, cache write, cache
read, output tokens), the per-turn log from stream-json (calls, tool-result characters, visible text, thinking and
tool-input characters), the final program, and the model's per-token prices (agentdx/pilot7_config.json). Nothing is
typed by hand: every dollar figure is tokens x price.

Thinking tokens. Claude Code's usage does not report thinking tokens separately: `output_tokens` includes them, and the
current models return thinking blocks with empty text by default. So `thinking_tokens_est` = output tokens minus the
visible output (assistant text plus tool-call inputs, characters / 3.5), floored at 0, and is marked an estimate.
"""
CHARS_PER_TOKEN = 3.5


def tokens(chars):
    return round((chars or 0) / CHARS_PER_TOKEN)


def decompose(split, turn_log, final_source, price, reported_usd=None):
    """split: {"fresh", "cache_write", "cache_read", "output"} totals; turn_log: stream turns; price: USD per million."""
    fresh, cw, cr, out = (split.get(k) or 0 for k in ("fresh", "cache_write", "cache_read", "output"))
    turns = turn_log or []
    visible = sum((t.get("text_chars") or 0) + (t.get("tool_input_chars") or 0) for t in turns)
    write_chars = sum(t.get("write_chars") or 0 for t in turns)
    visible_tok = tokens(visible)
    have_stream = any("text_chars" in t for t in turns)
    thinking = max(0, out - visible_tok) if have_stream else None
    usd = {
        "fresh_input": fresh * price["input"] / 1e6,
        "cache_write": cw * price["cache_write"] / 1e6,
        "cache_read": cr * price["cache_read"] / 1e6,
        "output_visible": min(out, visible_tok if have_stream else out) * price["output"] / 1e6,
        "output_thinking_est": (thinking or 0) * price["output"] / 1e6,
    }
    total = sum(usd.values())
    n_turns = len(turns) or None
    context_usd = usd["fresh_input"] + usd["cache_write"] + usd["cache_read"]
    return {
        "tokens": {"fresh_input": fresh, "cache_write": cw, "cache_read": cr, "output": out,
                   "output_visible_est": visible_tok if have_stream else None, "thinking_est": thinking,
                   "tool_result_est": tokens(sum(t.get("result_chars") or 0 for t in turns)),
                   "final_write_calls_est": tokens(write_chars) if have_stream else None},
        "turns": n_turns,
        "final_program_lines": sum(1 for l in (final_source or "").splitlines() if l.strip()),
        "final_program_tokens_est": tokens(len(final_source or "")),
        "usd": {k: round(v, 6) for k, v in usd.items()},
        "usd_total_computed": round(total, 6),
        "usd_reported": reported_usd,
        "usd_calibration": round(total / reported_usd, 3) if reported_usd else None,
        "usd_context_per_turn": round(context_usd / n_turns, 6) if n_turns else None,
        "thinking_is_estimate": True,
    }


def attribute_difference(a, b, price):
    """Split the mean cost difference b - a (two lists of decompositions) into context (input-side tokens), visible
    output, thinking, and the part of visible output that is the program itself (tokens in the final write calls).
    Also the number of turns, since every extra turn re-reads the context."""
    def mean(rows, f):
        vals = [f(r) for r in rows if f(r) is not None]
        return sum(vals) / len(vals) if vals else 0.0
    keys = {
        "context_usd": lambda r: r["usd"]["fresh_input"] + r["usd"]["cache_write"] + r["usd"]["cache_read"],
        "visible_output_usd": lambda r: r["usd"]["output_visible"],
        "thinking_usd_est": lambda r: r["usd"]["output_thinking_est"],
        "program_write_usd_est": lambda r: (r["tokens"]["final_write_calls_est"] or 0) * price["output"] / 1e6,
        "turns": lambda r: r["turns"],
        "program_lines": lambda r: r["final_program_lines"],
        "usd_total": lambda r: r["usd_total_computed"],
    }
    return {k: round(mean(b, f) - mean(a, f), 6) for k, f in keys.items()}


def token_saving_value(price, output_tokens=1000, context_tokens_per_turn=1000, turns=1):
    """What a pure token saving is worth at these prices: fewer output tokens, a smaller context re-read every turn
    (as cache reads), and one turn less (its context re-read plus one cache write of the new tail)."""
    return {"per_1k_output_tokens": round(output_tokens * price["output"] / 1e6, 6),
            "per_1k_context_tokens_over_turns": round(context_tokens_per_turn * turns * price["cache_read"] / 1e6, 6)}
