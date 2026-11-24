"""Ground-truth probe of Claude Code's don't-ask permission decisions with NO paid model call.

A local fake Anthropic Messages API (stdlib http.server on 127.0.0.1) answers the real `claude -p` with ONE Bash tool_use carrying the
command under test; Claude Code then decides allowed/denied with the exact `--allowedTools` the arm gets (arms.tool_policy). The decision
is read from the stream (`system`/`permission_denied` event). `python3 -m agentdx permission-probe` checks every case of
tests/fixtures/denials/probe_cases.json against the installed Claude Code and exits 1 when any decision differs from the recorded one
(a Claude Code upgrade that changes the rules, or an allow-set edit that opens/closes a shape).

`predict()` is the offline mirror of what the probe measured (no claude needed); tests assert predict() == the recorded decision for every
case, so the offline matcher is calibrated by the live one. Not modelled: wrappers such as `timeout N cmd` (Claude Code strips them and
checks the inner command; measured allowed for `timeout 10 wj run ...`).

Why Python: part of the Python harness (agentdx/*.py); needs an HTTP server and a subprocess, both stdlib. Python fallback: harness is
Python, no wj blocker."""
import concurrent.futures as cf
import http.server
import json
import os
import re
import shutil
import subprocess
import tempfile
import threading
import uuid

from . import arms as A, denials as DN
from .paths import ROOT

CASES = os.path.join(ROOT, "tests", "fixtures", "denials", "probe_cases.json")
CWD = "@CWD@"   # placeholder in a case for the probe's working directory (absolute path inside the cwd)


def load_cases(path=CASES):
    return json.load(open(path))["cases"]


def _outside(path, cwd="/work/trial"):
    """A path argument that leaves the trial directory: absolute and not under cwd, `..`, or `~`."""
    p = path.strip("'\"")
    if p.startswith(("~", "..")) or "/../" in p:
        return True
    return p.startswith("/") and not (p == cwd or p.startswith(cwd + "/") or p.startswith(CWD))


def allowed_rules(arm):
    return [x[5:-1] for x in A.tool_policy(arm)["allowed"] if x.startswith("Bash(")]   # e.g. "wj run:*", "printf:*", "./main:*"


def _covered(part, rules):
    words = part.split()
    if not words:
        return True
    for r in rules:
        prefix = r[:-2] if r.endswith(":*") else r
        if part == prefix or part.startswith(prefix + " "):
            return True
    return False


def predict(cmd, arm):
    """-> list of reasons Claude Code (2.1.331, measured) refuses `cmd` for `arm`; [] means allowed."""
    why = []
    sh = DN.shape(cmd)
    for m in ("space-eq", "subst", "heredoc"):
        if f"[{m}]" in sh:
            why.append(m)
    bare = re.sub(r"'[^']*'|\"[^\"]*\"", "''", cmd)
    if re.search(r"\d?>>?(?!&)(?!\s*/dev/null)", bare):   # measured: `2>&1`, `2>/dev/null`, `<`, `$?`, `$VAR` are accepted; a file redirect is not
        why.append("output redirect to a file")
    if re.search(r"(?<![&>])&(?![&>])", bare):
        why.append("background &")
    rules = allowed_rules(arm)
    for part, _ in DN.split_command(cmd):
        if re.match(r"[A-Za-z_]\w*=\S*\s+\S", part):
            why.append("leading VAR=value")
        part = re.sub(r"^timeout \d+ ", "", part)   # measured: Claude Code strips the wrapper and checks the inner command
        if not _covered(part, rules):
            why.append("not-allowed: " + " ".join(part.split()[:2]))
        w = part.split()
        if w[:1] == ["cd"] and len(w) > 1 and _outside(w[1]):
            why.append("cd outside the trial directory")
        if w[:1] == ["cat"] and any(_outside(x) for x in w[1:] if not x.startswith("-")):
            why.append("cat path outside the trial directory")
    return why


class _Fake(http.server.BaseHTTPRequestHandler):
    bash_cmd = ""

    def log_message(self, *a):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get("content-length", 0))) or b"{}")
        done = any(isinstance(m.get("content"), list) and any(c.get("type") == "tool_result" for c in m["content"]) for m in body.get("messages", []))
        if body.get("tools") and not done:
            blocks, stop = [{"type": "tool_use", "id": "toolu_" + uuid.uuid4().hex[:20], "name": "Bash", "input": {"command": self.bash_cmd, "description": "probe"}}], "tool_use"
        else:
            blocks, stop = [{"type": "text", "text": "done"}], "end_turn"
        mid = "msg_" + uuid.uuid4().hex[:12]
        if body.get("stream"):
            self.send_response(200)
            self.send_header("content-type", "text/event-stream")
            self.end_headers()

            def ev(t, d):
                self.wfile.write(f"event: {t}\ndata: {json.dumps(d)}\n\n".encode())
            ev("message_start", {"type": "message_start", "message": {"id": mid, "type": "message", "role": "assistant", "model": body.get("model", "m"), "content": [],
                                                                       "stop_reason": None, "stop_sequence": None, "usage": {"input_tokens": 1, "output_tokens": 1}}})
            for i, b in enumerate(blocks):
                if b["type"] == "text":
                    ev("content_block_start", {"type": "content_block_start", "index": i, "content_block": {"type": "text", "text": ""}})
                    ev("content_block_delta", {"type": "content_block_delta", "index": i, "delta": {"type": "text_delta", "text": b["text"]}})
                else:
                    ev("content_block_start", {"type": "content_block_start", "index": i, "content_block": {"type": "tool_use", "id": b["id"], "name": b["name"], "input": {}}})
                    ev("content_block_delta", {"type": "content_block_delta", "index": i, "delta": {"type": "input_json_delta", "partial_json": json.dumps(b["input"])}})
                ev("content_block_stop", {"type": "content_block_stop", "index": i})
            ev("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop, "stop_sequence": None}, "usage": {"output_tokens": 1}})
            ev("message_stop", {"type": "message_stop"})
        else:
            out = json.dumps({"id": mid, "type": "message", "role": "assistant", "model": body.get("model", "m"), "content": blocks,
                              "stop_reason": stop, "stop_sequence": None, "usage": {"input_tokens": 1, "output_tokens": 1}}).encode()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(out)))
            self.end_headers()
            self.wfile.write(out)


def decision_from_stream(text):
    """'denied' when the stream carries a permission_denied event, 'allowed' when a Bash tool_use ran without one, else 'error'."""
    den, order, uses = DN.scan_stream(text)
    if any(d["kind"] in ("dont-ask", "permission") for d in den):
        return "denied"
    return "allowed" if any(u[0] == "Bash" for u in uses.values()) else "error"


def probe(arm, cmd, claude_bin="claude", timeout=90):
    """-> 'allowed' | 'denied' | 'error' for one command under the arm's real allow list."""
    d = tempfile.mkdtemp(prefix="agentdx-permprobe-")
    try:
        cmd = cmd.replace(CWD, os.path.realpath(d))
        handler = type("H", (_Fake,), {"bash_cmd": cmd})
        srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=srv.serve_forever, daemon=True).start()
        pol = A.tool_policy(arm)
        env = dict(os.environ, HOME=d, ANTHROPIC_BASE_URL=f"http://127.0.0.1:{srv.server_address[1]}", ANTHROPIC_API_KEY="sk-fake-local-probe",
                   CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1")
        env.pop("ANTHROPIC_AUTH_TOKEN", None)
        argv = [claude_bin, "-p", "go", "--output-format", "stream-json", "--verbose", "--max-turns", "3", "--model", "haiku", "--permission-mode", pol["permission_mode"],
                "--tools", pol["tools"], "--allowedTools", ",".join(pol["allowed"]), "--setting-sources", "project", "--disable-slash-commands", "--no-session-persistence"]
        try:
            r = subprocess.run(argv, cwd=d, env=env, capture_output=True, text=True, timeout=timeout, stdin=subprocess.DEVNULL)
        except subprocess.TimeoutExpired:
            return "error"
        finally:
            srv.shutdown()
        return decision_from_stream(r.stdout)
    finally:
        shutil.rmtree(d, ignore_errors=True)


def run(args, say=print):
    claude = getattr(args, "claude", None) or shutil.which("claude")
    if not claude:
        say("RED   permissions\n      `claude` not found on PATH: the permission probe cannot run (set --claude)")
        return 1
    cases = load_cases(getattr(args, "cases", None) or CASES)
    with cf.ThreadPoolExecutor(max_workers=getattr(args, "jobs", 6)) as ex:
        got = list(ex.map(lambda c: probe(c["arm"], c["command"], claude), cases))
    bad = [f"{c['arm']}: `{c['command']}` expected {c['expect']}, Claude Code says {g} ({c.get('shape', '')})" for c, g in zip(cases, got) if g != c["expect"]]
    ver = subprocess.run([claude, "--version"], capture_output=True, text=True).stdout.strip()
    say(f"{'RED  ' if bad else 'GREEN'} permissions ({len(cases)} cases, {ver})" + "".join(f"\n      {b}" for b in bad))
    return 1 if bad else 0


def run_problems(args):
    """Preflight form: -> list of problems (empty = green)."""
    lines = []
    code = run(args, say=lines.append)
    return [] if code == 0 else [ln.strip() for ln in "\n".join(lines).splitlines()[1:]] or ["permission probe failed"]
