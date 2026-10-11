"""Agent-in-the-loop driver.

An *agent* is anything with `.turn(workspace, iteration, prompt_file, feedback_file) -> dict`
returning {"usage": {"input_tokens","output_tokens"}|None, "exit": int, "log": str}.
  ShellAgent  : pluggable shell command template (e.g. Claude Code, see agents/).
  ReplayAgent : writes recorded files (deterministic, no LLM) - see replay.py.
"""
import json
import os
import shlex
import shutil
import subprocess
import tempfile
import time

from . import grade as G
from . import metrics, workspace as W
from .paths import ROOT


class ShellAgent:
    """Template placeholders: {workspace} {prompt_file} {feedback_file} {lang} {task} {iteration} {src} {harness}.
    The command runs with cwd=workspace and env AGENTDX_USAGE_FILE (it may write
    {"input_tokens": n, "output_tokens": m} there) and AGENTDX_ITERATION."""

    def __init__(self, template, timeout=1800):
        self.template, self.timeout = template, timeout

    def turn(self, wd, iteration, prompt_file, feedback_file, lang, task):
        usage_file = os.path.join(wd, ".agentdx_usage.json")
        if os.path.exists(usage_file):
            os.remove(usage_file)
        cmd = self.template.format(workspace=shlex.quote(wd), prompt_file=shlex.quote(prompt_file),
                                   feedback_file=shlex.quote(feedback_file), lang=lang, task=task,
                                   iteration=iteration, src=W.src_name(lang), harness=shlex.quote(ROOT))
        env = dict(os.environ, AGENTDX_USAGE_FILE=usage_file, AGENTDX_ITERATION=str(iteration))
        try:
            p = subprocess.run(cmd, shell=True, cwd=wd, env=env, capture_output=True, text=True, timeout=self.timeout)
            code, log = p.returncode, (p.stdout + p.stderr)[-4000:]
        except subprocess.TimeoutExpired:
            code, log = -9, "agent timeout"
        usage = None
        if os.path.exists(usage_file):
            try:
                with open(usage_file) as uf:
                    usage = json.load(uf)
            except ValueError:
                usage = None
        return {"usage": usage, "exit": code, "log": log}


def read_events(log_path):
    if not os.path.exists(log_path):
        return []
    with open(log_path) as f:
        return [json.loads(l) for l in f if l.strip()]


def run_trial(task_id, lang, agent, max_iters=3, trial=0, keep_dir=None, regrade=True):
    """Run one trial. Returns a transcript dict (JSON-serializable)."""
    tmp = tempfile.mkdtemp(prefix="agentdx-trial-") if not keep_dir else keep_dir
    os.makedirs(tmp, exist_ok=True)
    wd = os.path.join(tmp, "ws")
    log_path = os.path.join(tmp, "checks.jsonl")
    W.make(task_id, lang, wd, log_path)
    prompt_file = os.path.join(wd, "PROMPT.md")
    feedback_file = os.path.join(wd, "FEEDBACK.md")
    iterations, seen_events = [], 0
    src_path = os.path.join(wd, W.src_name(lang))
    for i in range(1, max_iters + 1):
        t0 = time.time()
        res = agent.turn(wd, i, prompt_file, feedback_file, lang, task_id)
        wall = time.time() - t0
        source = ""
        if os.path.exists(src_path):
            with open(src_path) as sf:
                source = sf.read()
        vis = G.grade(lang, task_id, wd, hidden=False)
        hid = G.grade(lang, task_id, wd, hidden=True)
        evs = read_events(log_path)
        checks = [{"build_ok": e["build_ok"], "categories": e["categories"], "source": e["source"],
                   "samples_passed": e["samples_passed"]} for e in evs[seen_events:]]
        seen_events = len(evs)
        iterations.append({
            "index": i, "source": source, "wall_seconds": round(wall, 2), "usage": res["usage"],
            "agent_exit": res["exit"], "agent_log_tail": res["log"][-1500:], "agent_checks": checks,
            "build": {"ok": vis["build"]["ok"], "categories": sorted({d["category"] for d in vis["build"]["diagnostics"]}),
                      "diagnostics": [{"code": d["code"], "category": d["category"], "message": d["message"][:200]}
                                      for d in vis["build"]["diagnostics"][:10]]},
            "samples": {"passed": vis["passed"], "total": vis["total"], "status": vis["status"]},
            "hidden": {"passed": hid["passed"], "total": hid["total"], "status": hid["status"],
                       "categories": hid["categories"]},
        })
        if hid["status"] == "pass":
            break
        with open(feedback_file, "w") as f:
            f.write(W.feedback_text(vis))
    tr = {"schema": 1, "task": task_id, "lang": lang, "trial": trial, "max_iters": max_iters,
          "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%S"), "iterations": iterations}
    tr["metrics"] = metrics.trial_metrics(tr)
    if not keep_dir:
        shutil.rmtree(tmp, ignore_errors=True)
    return tr


def save(tr, directory):
    os.makedirs(directory, exist_ok=True)
    p = os.path.join(directory, f"{tr['task']}.{tr['lang']}.{tr['trial']}.json")
    with open(p, "w") as f:
        json.dump(tr, f, indent=1, sort_keys=True)
    return p
