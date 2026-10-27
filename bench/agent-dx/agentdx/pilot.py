"""Pilot runner: one agent session per trial (the agent iterates by itself), graded afterwards by the harness.

Record schema 2 per trial; see MODULE.md. The agent only ever sees `<trial>/ws` (starter main.wj) and the
prompt; the oracle, other trials and the shim log live outside it."""
import json
import os
import random
import re
import shutil
import subprocess
import time

from . import arms as A, budget as B, diagnostics as dg, grade as G, shim as S, workspace as W
from .runners import wj_bin

SCHEMA = 2
DENIAL_MIN_TRIALS, DENIAL_MAX_SHARE = 6, 0.5   # stop when the sandbox policy, not the language, blocks most trials
PILOT_TASKS = ["t003_prime_counts", "t006_base_convert", "t009_rpn_eval", "t011_lru_cache", "t013_bst",
               "t018_word_freq", "t022_csv_parse", "t030_bank_ledger", "t034_shapes_trait", "t050_cat_number"]


class ClaudeAgent:
    """Runs `claude -p` inside the sandbox with the wj shim first on PATH."""

    def __init__(self, model="sonnet", max_turns=25, max_trial_usd=2.0, claude_bin="claude", timeout=1500):
        self.model, self.max_turns, self.max_trial_usd = model, max_turns, max_trial_usd
        self.claude_bin, self.timeout = claude_bin, timeout

    def run(self, ws, shim_dir, arm, task):
        argv = A.claude_argv(arm, task, self.model, self.max_turns, self.max_trial_usd, self.claude_bin)
        env = dict(os.environ, PATH=shim_dir + os.pathsep + os.environ.get("PATH", ""),
                   CLAUDE_CODE_DISABLE_AUTO_MEMORY="1")
        env.pop("WJ_BIN", None)
        try:
            p = subprocess.run(argv, cwd=ws, env=env, capture_output=True, text=True, timeout=self.timeout)
            out, code = p.stdout, p.returncode
        except subprocess.TimeoutExpired:
            return {"json": None, "exit": -9, "log": "agent timeout"}
        try:
            j = json.loads(out)
        except ValueError:
            j = None
        return {"json": j, "exit": code, "log": (out if j is None else "")[-1500:]}


def _usage(j):
    if not j:
        return {"tokens_in": None, "tokens_out": None, "cost_usd": None, "turns": None, "api_seconds": None, "subtype": None, "permission_denials": None}
    u = j.get("usage") or {}
    tin = sum(u.get(k, 0) or 0 for k in ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"))
    return {"tokens_in": tin, "tokens_out": u.get("output_tokens", 0), "cost_usd": j.get("total_cost_usd"),
            "turns": j.get("num_turns"), "api_seconds": (j.get("duration_api_ms") or 0) / 1000.0,
            "subtype": j.get("subtype"), "is_error": j.get("is_error"),
            "permission_denials": len(j.get("permission_denials") or []),
            "denied_commands": [str((d.get("tool_input") or {}).get("command", d.get("tool_name")))[:200]
                                for d in (j.get("permission_denials") or [])][:5],
            "result_excerpt": str(j.get("result", ""))[:400]}


def analyse_invocations(log, lang="wj"):
    """Summaries of the shim log: per-verify-call table, invocations-to-green, mode violations, categories."""
    inv, cats = [], {}
    for e in log:
        row = {"verb": e["verb"], "mode": e["mode"], "exit": e["exit"], "secs": e["secs"]}
        if e["exit"] != 0 and e["verb"] in S.VERIFY_VERBS:
            ds = dg.parse_text(lang, e["out"])
            row["categories"] = sorted({d["category"] for d in ds})
            for c in row["categories"]:
                cats[c] = cats.get(c, 0) + 1
        inv.append(row)
    ver = [r for r in inv if r["verb"] in S.VERIFY_VERBS]
    green = None
    for i in range(len(ver) - 1, -1, -1):   # start of the final unbroken run of successes
        if ver[i]["exit"] == 0:
            green = i + 1
        else:
            break
    caps = sum(1 for e in log if re.search(r"WJ004\d|capabilit", e.get("out", "")) and e["exit"] != 0)
    return {"capability_denials": caps, "final_build_calls": sum(1 for r in inv if r["verb"] == "build"),
            "invocations": inv, "verify_calls": len(ver), "verify_calls_to_green": green,
            "tool_wait_seconds": round(sum(r["secs"] for r in inv), 2), "categories": cats,
            "first_source": next((e["source"] for e in log if e.get("source")), "")}


def violations(arm, inv):
    """Calls using the other arm's loop commands. D may build; native baselines have no modes."""
    loop = A.ARMS[arm]["loop"]
    if loop.startswith("native"):
        return 0
    if loop == "interpreted-plus-build":
        return sum(1 for r in inv if r["verb"] in S.VERIFY_VERBS and r["mode"] == "compiled" and r["verb"] != "build")
    bad = "compiled" if loop == "interpreted-first" else "interpret"
    return sum(1 for r in inv if r["verb"] in S.VERIFY_VERBS and r["mode"] == bad)


def default_grader(task, ws, lang="wj"):
    if lang != "wj":   # native baselines: one toolchain, so no interpreted/compiled divergence
        g = G.grade(lang, task, ws, hidden=True)
        return {"compiled": g, "interp": g}
    return {"compiled": G.grade("wj", task, ws, hidden=True), "interp": G.grade("wj-interp", task, ws, hidden=True)}


def error_excerpt(out, limit=600):
    """Error lines with a little context; cargo progress lines ("Compiling ...") are dropped."""
    lines = [l for l in out.splitlines() if not l.lstrip().startswith(("Compiling", "Finished", "Downloaded", "Blocking", "Running"))]
    idx = [i for i, l in enumerate(lines) if "error" in l.lower() or "panicked" in l]
    keep = []
    for i in idx[:4]:
        keep += lines[max(0, i - 0): i + 4]
    return ("\n".join(keep) if keep else "\n".join(lines))[:limit]


def _slim(g):
    return {"status": g["status"], "passed": g["passed"], "total": g["total"], "categories": g["categories"]}


def run_trial(task, arm, agent, trial, out_dir, grader=default_grader, real_wj=None):
    tdir = os.path.join(out_dir, "work", f"{task}.{arm}.{trial}")
    os.makedirs(tdir, exist_ok=True)
    ws = os.path.join(tdir, "ws")
    os.makedirs(ws, exist_ok=True)
    lang = A.ARMS[arm]["lang"]
    src = A.SRC[lang]
    with open(os.path.join(ws, src), "w") as f:
        f.write(W.starter(lang))
    log = os.path.join(tdir, "shim.jsonl")
    shim_dir = os.path.join(tdir, "shimbin")
    tool = A.SHIMMED[lang]
    real = (real_wj or wj_bin()) if lang == "wj" else shutil.which(tool)
    S.install(shim_dir, real, log, os.path.join(ws, src), tool)
    t0 = time.time()
    res = agent.run(ws, shim_dir, arm, task)
    wall = time.time() - t0
    final = open(os.path.join(ws, src)).read()
    g = default_grader(task, ws, lang) if grader is default_grader else grader(task, ws)
    an = analyse_invocations(S.read_log(log, tool), lang)
    u = _usage(res["json"])
    cpass, ipass = g["compiled"]["status"] == "pass", g["interp"]["status"] == "pass"
    api = u["api_seconds"] if u["api_seconds"] is not None else None
    tr = {"schema": SCHEMA, "task": task, "arm": arm, "arm_name": A.ARMS[arm]["name"], "trial": trial,
          "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
          "final_grade": _slim(g["compiled"]), "pass_final": cpass,
          "interp_grade": _slim(g["interp"]),
          "divergence": {"interp_pass": ipass, "compiled_pass": cpass, "diverged": ipass != cpass,
                         "silent": ipass and not cpass},
          "usage": u, "agent_exit": res["exit"], "agent_log": res["log"],
          "wall_seconds": round(wall, 2), "thinking_seconds": api,
          "tool_wait_seconds": an["tool_wait_seconds"],
          "other_seconds": None if api is None else round(max(0.0, wall - api - an["tool_wait_seconds"]), 2),
          "lang": lang, "capability_denials": an["capability_denials"], "final_build_calls": an["final_build_calls"],
          "invocations": an["invocations"], "verify_calls": an["verify_calls"],
          "verify_calls_to_green": an["verify_calls_to_green"], "diagnostic_categories": an["categories"],
          "mode_violations": violations(arm, an["invocations"]),
          "first_source": an["first_source"], "final_source": final,
          "final_build_diag": [d["message"] for d in g["compiled"].get("build", {}).get("diagnostics", [])][:3],
          "diag_samples": [error_excerpt(e["out"]) for e in S.read_log(log, tool) if e["exit"] != 0 and e["verb"] in S.VERIFY_VERBS][:12]}
    for name in os.listdir(ws):     # build outputs can be large; the record keeps first and final sources
        if name != src:
            q = os.path.join(ws, name)
            shutil.rmtree(q, ignore_errors=True) if os.path.isdir(q) else os.remove(q)
    return tr


def plan(tasks, arm_ids, trials, seed=0):
    """Round-robin by trial index so an abort leaves the arms balanced; order shuffled within a round."""
    rng = random.Random(seed)
    out = []
    for k in range(trials):
        rnd = [(t, a, k) for t in tasks for a in arm_ids]
        rng.shuffle(rnd)
        out += rnd
    return out


def save(tr, out_dir):
    d = os.path.join(out_dir, "trials")
    os.makedirs(d, exist_ok=True)
    p = os.path.join(d, f"{tr['task']}.{tr['arm']}.{tr['trial']}.json")
    with open(p, "w") as f:
        json.dump(tr, f, indent=1, sort_keys=True)
    return p


def load_trials(out_dir):
    d = os.path.join(out_dir, "trials")
    if not os.path.isdir(d):
        return []
    return [json.load(open(os.path.join(d, n))) for n in sorted(os.listdir(d)) if n.endswith(".json")]


def run_pilot(agent, tasks, arm_ids, trials, out_dir, guard, grader=default_grader, real_wj=None, say=print, workers=1):
    """Run the plan; stop (return the reason) when the guard trips. Resumed runs count earlier trials."""
    from concurrent.futures import ThreadPoolExecutor
    for prev in load_trials(out_dir):
        guard.count += 1
        guard.spent += (prev.get("usage") or {}).get("cost_usd") or 0.0
    todo = [(t, a, k) for t, a, k in plan(tasks, arm_ids, trials)
            if not os.path.exists(os.path.join(out_dir, "trials", f"{t}.{a}.{k}.json"))]
    done, reasons, denied = [], [], []

    def one(item):
        task, arm, k = item
        if reasons:
            return None
        try:
            guard.before_trial()
        except B.BudgetExceeded as e:
            reasons.append(str(e))
            return None
        try:
            tr = run_trial(task, arm, agent, k, out_dir, grader, real_wj)
        except BaseException:
            guard.record(None)
            raise
        guard.record(tr["usage"]["cost_usd"])
        if tr["usage"].get("permission_denials"):
            denied.append(task)
        n_done = len(done) + 1
        if n_done >= DENIAL_MIN_TRIALS and len(denied) / n_done > DENIAL_MAX_SHARE:
            reasons.append(f"{len(denied)}/{n_done} trials hit harness permission denials (invalid measurements): {tr['usage']['denied_commands'][:2]}")
        done.append(save(tr, out_dir))
        say(f"{task} arm={arm} trial={k} pass={tr['pass_final']} turns={tr['usage']['turns']} "
            f"cost={tr['usage']['cost_usd']} spent=${guard.spent:.2f}")
        return tr
    if workers <= 1:
        for it in todo:
            one(it)
            if reasons:
                break
    else:
        with ThreadPoolExecutor(workers) as ex:
            list(ex.map(one, todo))
    return {"completed": len(done), "aborted_reason": reasons[0] if reasons else None, "spent_usd": round(guard.spent, 4)}


def regrade_failures(out_dir, grader=default_grader, say=print):
    """Re-grade every non-passing trial from its recorded final source (sequentially). Passing trials are kept as is:
    a pass cannot come from a wrong binary. Original grades are preserved under `original_*`."""
    changed = 0
    for tr in load_trials(out_dir):
        if tr["pass_final"] or tr.get("regraded"):
            continue
        ws = os.path.join(out_dir, "regrade", f"{tr['task']}.{tr['arm']}.{tr['trial']}")
        os.makedirs(ws, exist_ok=True)
        lang = tr.get("lang", "wj")
        with open(os.path.join(ws, A.SRC[lang]), "w") as f:
            f.write(tr["final_source"])
        g = default_grader(tr["task"], ws, lang) if grader is default_grader else grader(tr["task"], ws)
        shutil.rmtree(ws, ignore_errors=True)
        tr["original_final_grade"], tr["original_interp_grade"] = tr["final_grade"], tr["interp_grade"]
        cpass, ipass = g["compiled"]["status"] == "pass", g["interp"]["status"] == "pass"
        tr["final_grade"], tr["interp_grade"], tr["pass_final"] = _slim(g["compiled"]), _slim(g["interp"]), cpass
        tr["divergence"] = {"interp_pass": ipass, "compiled_pass": cpass, "diverged": ipass != cpass, "silent": ipass and not cpass}
        tr["final_build_diag"] = [d["message"] + "\n" + (d.get("detail") or "")[:300] for d in g["compiled"].get("build", {}).get("diagnostics", [])][:3]
        tr["regraded"] = True
        changed += tr["original_final_grade"] != tr["final_grade"]
        save(tr, out_dir)
        say(f"regraded {tr['task']} {tr['arm']} {tr['trial']}: pass={cpass} (was {tr['original_final_grade']['status']})")
    return changed
