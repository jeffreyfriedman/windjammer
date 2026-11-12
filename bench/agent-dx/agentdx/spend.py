"""Spend accounting for paid runs: an append-only, fsynced ledger and cost recovery from a (partial) stream-json log.

Why Python: this extends the existing Python harness (agentdx/*.py); porting the whole harness to Windjammer is a
separate item. Python fallback: harness is Python, no wj blocker.

Ledger (<out>/spend.jsonl), one JSON object per line, each fsynced before the trial proceeds:
  {"ev": "start", "attempt": id, "trial": "<model>/<task>.<arm>.<k>", "model": m, "cap_usd": c, "stream": path, "t": iso}
  {"ev": "end",   "attempt": id, "usd": x, "status": s, "basis": b, "t": iso}
An attempt with a start and no end is *unfinished* (process killed): it counts at its reserved cap until `reconcile`
finds a stream log for it. Every attempt counts, including those of trials that were later re-run."""
import json
import os
import time

from . import stream as ST


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S")


class Ledger:
    def __init__(self, path):
        self.path = path
        os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)

    def _append(self, obj):
        fd = os.open(self.path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
        try:
            os.write(fd, (json.dumps(obj, sort_keys=True) + "\n").encode())
            os.fsync(fd)
        finally:
            os.close(fd)

    def start(self, trial, model, cap_usd, stream_path):
        attempt = f"{trial}@{time.time_ns()}"
        self._append({"ev": "start", "attempt": attempt, "trial": trial, "model": model, "cap_usd": cap_usd, "stream": stream_path, "t": now()})
        return attempt

    def end(self, attempt, usd, status, basis):
        self._append({"ev": "end", "attempt": attempt, "usd": usd, "status": status, "basis": basis, "t": now()})

    def attempts(self):
        """-> list of {attempt, trial, model, cap_usd, stream, usd (None when unfinished), status, basis}, in start order."""
        out, by = [], {}
        if not os.path.exists(self.path):
            return out
        for ln in open(self.path):
            try:
                e = json.loads(ln)
            except ValueError:
                continue   # a torn last line from a kill: the start line before it was fsynced
            if e.get("ev") == "start":
                by[e["attempt"]] = dict(e, usd=None, status="unfinished", basis="reserved_cap")
                out.append(by[e["attempt"]])
            elif e.get("ev") == "end" and e.get("attempt") in by:
                by[e["attempt"]].update(usd=e["usd"], status=e["status"], basis=e["basis"])
        return out

    def charged(self, a):
        return a["cap_usd"] if a["usd"] is None else a["usd"]

    def reconcile(self, prices=None, say=print):
        """Settle unfinished attempts from their stream log when one exists; returns the number settled."""
        n = 0
        for a in self.attempts():
            if a["usd"] is not None or not a.get("stream") or not os.path.exists(a["stream"]):
                continue
            usd, basis = cost_from_stream(open(a["stream"], errors="replace").read(), (prices or {}).get(a["model"]))
            if usd is None:
                continue
            self.end(a["attempt"], usd, "interrupted", basis + "_reconciled")
            say(f"ledger: reconciled unfinished {a['trial']} at ${usd:.4f} ({basis})")
            n += 1
        return n


def price_turns(log, price):
    """Dollars for a per-turn log (stream.parse) at a price row. Cache writes use the 1-hour rate when the table has it
    (the higher one, so a partial log is not under-charged)."""
    w = price.get("cache_write_1h", price["cache_write"])
    return sum(t["fresh"] * price["input"] + t["cache_write"] * w + t["cache_read"] * price["cache_read"] + t["output"] * price["output"]
               for t in log) / 1e6


def cost_from_stream(text, price):
    """-> (usd | None, basis). Claude Code's own total_cost_usd when the result event is there; else the per-turn usage
    priced at `price` (None price: no estimate); else (None, 'none') and the caller charges the per-trial cap."""
    result, log, _ = ST.parse(text or "")
    if result and result.get("total_cost_usd") is not None:
        return float(result["total_cost_usd"]), "result"
    if log and price:
        return price_turns(log, price), "partial_stream"
    return None, "none"
