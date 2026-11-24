"""Harness permission denials: detect them in a trial's `claude` output and describe what was denied.

Why Python: this extends the existing Python harness (agentdx/*.py); porting the whole harness to Windjammer is a
separate item. Python fallback: harness is Python, no wj blocker.

Three kinds are counted, from two sources (the stream's tool results, and the final result JSON's `permission_denials`):
  dont-ask    a tool result "Permission to use Bash has been denied because Claude Code is running in don't ask mode ..."
              (what `--permission-mode dontAsk` returns for a command that is not fully covered by --allowedTools; for a
              compound command EVERY part must be allowed, so one unlisted part denies the whole line)
  permission  any other "Permission to use X has been denied" tool result
  capability  a tool result carrying a WJ004x / capability-enforcement diagnostic (the program's own sandbox, not the harness)
The result JSON's `permission_denials` list ([{"tool_name", "tool_use_id", "tool_input": {"command": ...}}]) is merged in by
tool_use_id, so a denial seen in only one source is still counted once.

A command's SHAPE is its sub-commands in order, joined by the operator that followed them, plus markers for redirects,
substitutions and heredocs: `wj | head && printf | ./main [redirect]`. Shapes tell which part of a compound line to allow."""
import json
import re

HAS_BEEN_DENIED = re.compile(r"has been denied", re.I)
DONT_ASK = re.compile(r"don['’]?t ask", re.I)
CAPABILITY = re.compile(r"WJ004\d|capabilit", re.I)
OPS = ("&&", "||", ";", "|", "\n")


def classify_text(text):
    """-> 'dont-ask' | 'permission' | 'capability' | None for one tool result's text."""
    if HAS_BEEN_DENIED.search(text) and DONT_ASK.search(text):
        return "dont-ask"
    if re.search(r"Permission to use \S+ has been denied", text):
        return "permission"
    if CAPABILITY.search(text):
        return "capability"
    return None


def split_command(cmd):
    """Quote-aware split into [(part, operator_after)]; no shell semantics beyond quotes and backslashes."""
    parts, cur, q, i = [], [], None, 0
    while i < len(cmd):
        ch = cmd[i]
        if q:
            cur.append(ch)
            if ch == "\\" and q == '"' and i + 1 < len(cmd):
                i += 1
                cur.append(cmd[i])
            elif ch == q:
                q = None
        elif ch in "'\"":
            q = ch
            cur.append(ch)
        elif ch == "\\" and i + 1 < len(cmd):
            cur.append(ch + cmd[i + 1])
            i += 1
        else:
            op = next((o for o in OPS if cmd.startswith(o, i)), None)
            if op:
                parts.append(("".join(cur).strip(), op if op != "\n" else ";"))
                cur = []
                i += len(op) - 1
            else:
                cur.append(ch)
        i += 1
    parts.append(("".join(cur).strip(), ""))
    return [(p, o) for p, o in parts if p]


def _word(part):
    for w in part.split():
        if "=" in w and not w.startswith(("=", "-", "./", "/")) and re.match(r"^[A-Za-z_]\w*=", w):
            continue   # leading VAR=value
        return w
    return ""


def sub_commands(cmd):
    """First word of every part of a (compound) command, in order."""
    return [_word(p) for p, _ in split_command(cmd) if _word(p)]


def shape(cmd):
    marks = []
    if re.search(r"(^|[^\\])\$\(|`", cmd):
        marks.append("subst")
    if re.search(r"\$[A-Za-z_?!{]", cmd):
        marks.append("var")
    if "<<" in cmd:
        marks.append("heredoc")
    stripped = re.sub(r"'[^']*'|\"[^\"]*\"", "''", cmd)
    if re.search(r"\d?>|<", stripped.replace("<<", "")):
        marks.append("redirect")
    if re.search(r" =", cmd):
        marks.append("space-eq")   # Claude Code refuses a space followed by `=` anywhere in a Bash line, even inside quotes (zsh `=cmd` expansion check)
    if any(re.search(r"[<>]", q) for q in re.findall(r"'[^']*'|\"[^\"]*\"", cmd)):
        marks.append("quoted-redirect-char")
    out = ""
    for p, op in split_command(cmd):
        out += (_word(p) or "?") + (f" {op} " if op else "")
    return out.strip() + ("".join(f" [{m}]" for m in marks))


def _text_of(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(c.get("text", "") if isinstance(c, dict) else str(c) for c in content)
    return ""


def scan_stream(text):
    """-> list of {"id", "tool", "command", "kind", "source"} for each denied tool call in a stream-json log."""
    uses, found = {}, {}
    order = []
    for ln in text.splitlines():
        ln = ln.strip()
        if not ln.startswith("{"):
            continue
        try:
            ev = json.loads(ln)
        except ValueError:
            continue
        t = ev.get("type")
        msg = ev.get("message")
        # `message` is a dict on assistant/user events and a plain string on system events.
        content = msg.get("content") if isinstance(msg, dict) else None
        if t == "system" and ev.get("subtype") == "permission_denied":
            # Claude Code's own denial event: exact (tool_use_id + reason), no text matching needed.
            tid = ev.get("tool_use_id") or f"system-denial-{len(found)}"
            found[tid] = "dont-ask" if ev.get("decision_reason_type") == "mode" else "permission"
            uses.setdefault(tid, (ev.get("tool_name", "?"), ""))
            continue
        if t == "assistant" and isinstance(content, list):
            for c in content:
                if isinstance(c, dict) and c.get("type") == "tool_use":
                    inp = c.get("input") or {}
                    uses[c.get("id")] = (c.get("name", "?"), str(inp.get("command", "")))
                    order.append(c.get("id"))
        elif t == "user" and isinstance(content, list):
            for c in content:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    kind = classify_text(_text_of(c.get("content")))
                    if kind:
                        found[c.get("tool_use_id")] = kind
        elif t == "result":
            for n, d in enumerate(ev.get("permission_denials") or []):
                d = dict(d, tool_use_id=d.get("tool_use_id") or f"result-denial-{n}")
                found.setdefault(d["tool_use_id"], "dont-ask")
                uses.setdefault(d["tool_use_id"], (d.get("tool_name", "?"), str((d.get("tool_input") or {}).get("command", ""))))
    return [{"id": i, "tool": uses.get(i, ("?", ""))[0], "command": uses.get(i, ("?", ""))[1], "kind": k} for i, k in found.items()], order, uses


def first_bash_denied(text):
    """True when the first Bash call of the stream was denied (permission kinds only)."""
    denied, order, uses = scan_stream(text)
    ids = {d["id"] for d in denied if d["kind"] in ("dont-ask", "permission")}
    for i in order:
        if uses[i][0] == "Bash":
            return i in ids
    return False


def summarise(stream_text, result_json=None):
    """The per-trial denial record. result_json's permission_denials are merged in when the stream lacks them."""
    stream_text = stream_text or ""
    if result_json and result_json.get("permission_denials"):   # merged by tool_use_id: a denial in both sources counts once
        stream_text += "\n" + json.dumps(dict(result_json, type="result"))
    denied, order, uses = scan_stream(stream_text)
    perm = [d for d in denied if d["kind"] in ("dont-ask", "permission")]
    kinds = {}
    for d in denied:
        kinds[d["kind"]] = kinds.get(d["kind"], 0) + 1
    shapes = {}
    for d in perm:
        s = shape(d["command"]) if d["tool"] == "Bash" else d["tool"]
        shapes[s] = shapes.get(s, 0) + 1
    first = next((i for i in order if uses[i][0] == "Bash"), None)
    return {"count": len(perm), "capability": kinds.get("capability", 0), "kinds": kinds, "shapes": shapes,
            "commands": [d["command"][:300] for d in perm][:20],
            "first_bash_denied": first is not None and any(d["id"] == first for d in perm)}


PROGRAM_FILE = re.compile(r"(^|[\\/])(main\.(wj|rs|py|go)|[A-Za-z0-9_]+_test\.wj|[A-Za-z0-9_]+\.(wj|rs|py|go))$")


def input_writes(stream_text):
    """Number of Write calls that create something other than program source: the input files the prompt rule asks for when program
    input contains a space before `=`, `$`, `<` or `>`. This is the rule's turn cost, reported per arm so it is visible in every pilot."""
    n = 0
    for ln in (stream_text or "").splitlines():
        ln = ln.strip()
        if not ln.startswith("{"):
            continue
        try:
            ev = json.loads(ln)
        except ValueError:
            continue
        msg = ev.get("message")
        content = msg.get("content") if isinstance(msg, dict) else None
        if ev.get("type") == "assistant" and isinstance(content, list):
            for c in content:
                if isinstance(c, dict) and c.get("type") == "tool_use" and c.get("name") == "Write":
                    if not PROGRAM_FILE.search(str((c.get("input") or {}).get("file_path", ""))):
                        n += 1
    return n
