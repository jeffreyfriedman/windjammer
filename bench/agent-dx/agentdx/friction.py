"""Friction list: what made agents stumble, mapped to a concrete fix location and ranked by estimated savings.

Signals come from three places: (1) failing `wj` invocations (their output tails), (2) interpreter/compiled
divergence, (3) slow commands. Each signal is matched by RULES; unmatched failures are grouped by a normalised
first diagnostic line so new friction shows up without editing the rules. Savings = wasted verify calls x the
arm's mean tokens per turn: an estimate, shown as such.
"""
import re
from collections import defaultdict

# id, regex, title, fix kind, fix location
RULES = [
    ("lint-leakage", r"W00(0[1-9]|1\d)", "Rust-leakage lint rejected idiomatic-looking code", "lint rule",
     "src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms'"),
    ("stdin-api", r"read_line|read_to_string|stdin", "stdin API differs between interpreter and compiled path", "stdlib API",
     "std::io unification; pack section 'Reading input'"),
    ("interp-unsupported", r"unsupported in the interpreter|not implemented by the interpreter", "interpreter refused a feature the compiled path accepts",
     "interpreter coverage", "src/interpreter + docs/design/interpreter-primary-readiness.md R6"),
    ("name-unknown-method", r"no method named|unknown method|has no method", "method guessed from Rust/Python does not exist", "stdlib API / docs",
     "std item or alias; `wj docs` index; error text should suggest the nearest real name"),
    ("name-unresolved", r"cannot find|unresolved|unknown (name|function|variable)|not found in scope", "unresolved name", "diagnostic text",
     "diagnostic should list candidates and the import to add"),
    ("type-mismatch", r"mismatched types|type mismatch|expected .* found", "type mismatch (often int/float, string/&str)", "diagnostic text",
     "show both types and the conversion (`as`, `.to_string()`); error-code explain entry"),
    ("rustc-leak", r"error\[E0\d{3}\]|--> src/main\.rs|rustc|cargo", "raw rustc/cargo error surfaced from generated code", "codegen / diagnostic mapping",
     "map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5)"),
    ("parse-error", r"parse error|Expected .* got|unexpected token", "parse error with weak guidance", "diagnostic text / parser",
     "parser error recovery message with a 'did you mean' for the construct"),
    ("runtime-panic", r"panicked at|exit status 101", "runtime panic during the loop", "docs", "pack section on Option/Result handling and index bounds"),
]
SLOW_SECS = 30.0
KNOWN_SEEDS = [  # from docs/known-issues/agent-dx-*.md; shown with zero observed weight until a run hits them
    ("None-first-arm", "interpreter: `None` as first match arm matches everything (silent divergence)", "interpreter",
     "docs/known-issues/agent-dx-interp-none-first-match-arm.md"),
    ("entry-binding", "interpreter: binding to entry().or_insert loses mutation (silent divergence)", "interpreter",
     "docs/known-issues/agent-dx-interp-entry-or-insert-binding-loses-mutation.md"),
    ("stdin-read-line", "compiled: stdin read_line(line) never terminates", "stdlib API",
     "docs/known-issues/agent-dx-stdin-apis-differ-between-modes.md"),
]


def normalise(text):
    first = next((ln for ln in text.splitlines() if re.search(r"error|Error|panick|warning", ln)), text.strip().split("\n")[0] if text.strip() else "")
    first = re.sub(r"\b\d+\b", "N", first)
    first = re.sub(r"`[^`]*`", "`X`", first)
    return first.strip()[:140]


def _match(text):
    for rid, rx, title, kind, where in RULES:
        if re.search(rx, text):
            return rid, title, kind, where
    return None


def build(trials, tokens_per_turn=None):
    """-> ranked list of friction dicts. `trials` are schema-2 records (any arms)."""
    toks = defaultdict(list)
    for t in trials:
        u = t["usage"]
        if u.get("tokens_in") and u.get("turns"):
            toks[t["arm"]].append((u["tokens_in"] + (u.get("tokens_out") or 0)) / u["turns"])
    per_turn = tokens_per_turn or {a: sum(v) / len(v) for a, v in toks.items()}
    default_tpt = (sum(per_turn.values()) / len(per_turn)) if per_turn else 0.0
    items = {}

    def add(key, title, kind, where, trial, wasted, arm, example=""):
        it = items.setdefault(key, {"id": key, "title": title, "fix_kind": kind, "fix_location": where,
                                    "trials": set(), "tasks": set(), "wasted_calls": 0, "est_tokens": 0.0, "example": example, "arms": set()})
        it["trials"].add((trial["task"], trial["arm"], trial["trial"]))
        it["tasks"].add(trial["task"])
        it["arms"].add(trial["arm"])
        it["wasted_calls"] += wasted
        it["est_tokens"] += wasted * per_turn.get(arm, default_tpt)
        if not it["example"] and example:
            it["example"] = example[:300]

    for t in trials:
        arm = t["arm"]
        for s in t.get("diag_samples", []):
            m = _match(s)
            if m:
                add(m[0], m[1], m[2], m[3], t, 1, arm, s)
            else:
                n = normalise(s)
                if n:
                    add("other:" + n, "unclassified failure: " + n, "diagnostic text (triage)", "inspect example, then file under a rule", t, 1, arm, s)
        d = t.get("divergence", {})
        if d.get("silent"):
            add("silent-divergence", "interpreted run passed but compiled grade failed (silent divergence)", "interpreter / check parity",
                "R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus", t,
                max(1, (t.get("verify_calls") or 1) // 2), arm)
        for r in t.get("invocations", []):
            if r["secs"] >= SLOW_SECS:
                add("slow-command:" + r["verb"] + "-" + r["mode"], f"slow `wj {r['verb']}` ({r['mode']}) >= {SLOW_SECS:.0f}s", "performance",
                    "build cache / runtime pre-build; see wall-time table", t, 1, arm)
    out = []
    for it in items.values():
        it["n_trials"], it["n_tasks"] = len(it["trials"]), len(it["tasks"])
        it["arms"] = sorted(it["arms"])
        del it["trials"], it["tasks"]
        it["est_tokens"] = round(it["est_tokens"])
        out.append(it)
    out.sort(key=lambda x: (-x["est_tokens"], -x["wasted_calls"], x["id"]))
    for i, it in enumerate(out, 1):
        it["rank"] = i
        it["priority"] = "P0" if i <= 3 else "P1" if i <= 8 else "P2"
    return out
