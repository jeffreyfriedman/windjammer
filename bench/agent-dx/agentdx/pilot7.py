"""Pilot 7 (ADR-0037): models x arms x tasks x trials, with a dollar guard per model and a hard cap, and a cost
decomposition per trial. `dry_run` checks the whole matrix without any LLM call.

Configuration: agentdx/pilot7_config.json (models with per-trial caps and expected cost, the price table with its date,
arms, tasks, trials per cell, hard cap). Output: <out>/<model>/trials/<task>.<arm>.<trial>.json (schema 2 plus `model`
and `cost_decomposition`)."""
import json
import os
import shutil
import tempfile

from . import arms as A, budget as B, costdecomp as CD, pilot as PL, tasks as T, workspace as W
from .paths import ROOT

CONFIG = os.path.join(os.path.dirname(__file__), "pilot7_config.json")


def load_config(path=CONFIG):
    return json.load(open(path))


def tasks_dir(cfg):
    return os.path.join(ROOT, cfg["tasks_dir"])


def use_tasks(cfg):
    """Point task loading (and child processes) at the pilot's task directory."""
    os.environ["AGENTDX_TASKS_DIR"] = tasks_dir(cfg)
    T.TASKS = tasks_dir(cfg)


def matrix(cfg):
    """[(model, task, arm, trial)] in run order: cheapest model first, round-robin by trial inside a model."""
    out = []
    for model in cfg["models"]:
        out += [(model, t, a, k) for t, a, k in PL.plan(cfg["tasks"], cfg["arms"], cfg["trials_per_cell"])]
    return out


class MatrixGuard:
    """Hard cap over the whole matrix, and a projection per model: after `check_after` trials of a model, the spend so
    far plus that model's remaining trials at its observed mean plus every later model at its expected cost must stay
    under the cap, or the model's remaining trials are skipped (the cheaper models' results stay valid)."""

    def __init__(self, cfg, plan_rows):
        self.cap, self.check_after = cfg["hard_cap_usd"], cfg["check_after_trials_per_model"]
        self.expected = {m: v["expected_usd_per_trial"] for m, v in cfg["models"].items()}
        self.trial_cap = {m: v["per_trial_usd_cap"] for m, v in cfg["models"].items()}
        self.planned = {m: sum(1 for r in plan_rows if r[0] == m) for m in cfg["models"]}
        self.done = {m: 0 for m in cfg["models"]}
        self.spent_by = {m: 0.0 for m in cfg["models"]}
        self.aborted = {}

    @property
    def spent(self):
        return sum(self.spent_by.values())

    def mean(self, m):
        return self.spent_by[m] / self.done[m] if self.done[m] else self.expected[m]

    def projection(self):
        return self.spent + sum((self.planned[m] - self.done[m]) * (self.mean(m) if self.done[m] >= self.check_after else self.expected[m])
                                for m in self.planned if m not in self.aborted)

    def allow(self, m):
        """-> None to run the next trial of model m, or the reason it is skipped."""
        if m in self.aborted:
            return self.aborted[m]
        if self.spent + self.trial_cap[m] > self.cap:
            self.aborted[m] = f"hard cap: spent ${self.spent:.2f} + one {m} trial at its cap ${self.trial_cap[m]:.2f} > ${self.cap:.2f}"
            return self.aborted[m]
        if self.done[m] >= self.check_after and self.projection() > self.cap:
            self.aborted[m] = (f"projection after {self.done[m]} {m} trials (mean ${self.mean(m):.3f}): ${self.projection():.2f} "
                               f"> cap ${self.cap:.2f}; {m} stopped")
            return self.aborted[m]
        return None

    def record(self, m, usd):
        self.done[m] += 1
        self.spent_by[m] += usd or 0.0


def decompose_trial(tr, cfg):
    price = cfg["prices"][tr["model"]]
    u = tr.get("usage") or {}
    split = u.get("split") or {"fresh": 0, "cache_write": 0, "cache_read": 0, "output": u.get("tokens_out") or 0}
    return CD.decompose(split, tr.get("turn_log") or [], tr.get("final_source", ""), price, u.get("cost_usd"))


def run_matrix(cfg, out_dir, agent_for, say=print, grader=PL.default_grader, real_wj=None):
    """agent_for(model) -> agent with .run(); one trial at a time (pilot 6: parallel trials shared a binary)."""
    use_tasks(cfg)
    rows = matrix(cfg)
    g = MatrixGuard(cfg, rows)
    for m in cfg["models"]:           # resume: count trials already on disk
        for tr in PL.load_trials(os.path.join(out_dir, m)):
            g.record(m, (tr.get("usage") or {}).get("cost_usd"))
    agents = {}
    for m, task, arm, k in rows:
        mdir = os.path.join(out_dir, m)
        if os.path.exists(os.path.join(mdir, "trials", f"{task}.{arm}.{k}.json")):
            continue
        why = g.allow(m)
        if why:
            continue
        agents.setdefault(m, agent_for(m))
        tr = PL.run_trial(task, arm, agents[m], k, mdir, grader, real_wj)
        tr["model"] = m
        tr["cost_decomposition"] = decompose_trial(tr, cfg)
        g.record(m, tr["usage"]["cost_usd"])
        PL.save(tr, mdir)
        say(f"{m} {task} {arm} {k} pass={tr['pass_final']} turns={tr['usage']['turns']} cost={tr['usage']['cost_usd']} spent=${g.spent:.2f}")
    return {"spent_usd": round(g.spent, 4), "done": g.done, "aborted": g.aborted}


def _check(ok, msg, problems):
    if not ok:
        problems.append(msg)


def dry_run(cfg, prior_trials_dir=None, say=print):
    """Everything short of calling a model. Returns the list of problems (empty = ready)."""
    before = (os.environ.get("AGENTDX_TASKS_DIR"), T.TASKS)
    try:
        return _dry_run(cfg, prior_trials_dir, say)
    finally:
        T.TASKS = before[1]
        if before[0] is None:
            os.environ.pop("AGENTDX_TASKS_DIR", None)
        else:
            os.environ["AGENTDX_TASKS_DIR"] = before[0]


def _dry_run(cfg, prior_trials_dir, say):
    problems = []
    use_tasks(cfg)
    for m in cfg["models"]:
        _check(m in cfg["prices"] and all(k in cfg["prices"][m] for k in ("input", "cache_write", "cache_read", "output")),
               f"model {m}: no complete price row", problems)
    for a in cfg["arms"]:
        _check(a in A.ARMS, f"arm {a}: not in agentdx/arms.py", problems)
        pk = A.pack_path(a) if a in A.ARMS else None
        _check(pk is None or os.path.exists(pk), f"arm {a}: pack {pk} missing", problems)
    _check(len(cfg["tasks"]) >= 8, f"{len(cfg['tasks'])} tasks configured; the plan asks for about 10", problems)
    for t in cfg["tasks"]:
        d = os.path.join(tasks_dir(cfg), t)
        for f in ("task.json", "spec.md", "samples.json", "oracle/vectors.json"):
            _check(os.path.exists(os.path.join(d, f)), f"task {t}: {f} missing", problems)
    rows = matrix(cfg)
    n = len(rows)
    _check(n <= B.PILOT_MAX_TRIALS, f"{n} trials exceed the harness ceiling {B.PILOT_MAX_TRIALS}", problems)
    say(f"matrix: {len(cfg['models'])} models x {len(cfg['arms'])} arms x {len(cfg['tasks'])} tasks x {cfg['trials_per_cell']} trials = {n} trials")
    exp = {m: sum(1 for r in rows if r[0] == m) * v["expected_usd_per_trial"] for m, v in cfg["models"].items()}
    worst = {m: sum(1 for r in rows if r[0] == m) * v["per_trial_usd_cap"] for m, v in cfg["models"].items()}
    for m in cfg["models"]:
        say(f"  {m}: {sum(1 for r in rows if r[0] == m)} trials, expected ${exp[m]:.2f}, every trial at its cap ${worst[m]:.2f} "
            f"(per-trial cap ${cfg['models'][m]['per_trial_usd_cap']:.2f}, price table {cfg['prices']['_date']}: {cfg['prices'][m]['model']})")
    say(f"  expected total ${sum(exp.values()):.2f}; hard cap ${cfg['hard_cap_usd']:.2f}; projection checked after {cfg['check_after_trials_per_model']} trials per model")
    _check(sum(exp.values()) <= cfg["hard_cap_usd"], "the expected total exceeds the hard cap", problems)
    # workspaces and argv, exactly as a live trial builds them (no model is called)
    tmp = tempfile.mkdtemp(prefix="p7dry-")
    try:
        for t in [t for t in cfg["tasks"] if os.path.exists(os.path.join(tasks_dir(cfg), t, "task.json"))]:
            for a in [a for a in cfg["arms"] if a in A.ARMS]:
                lang = A.ARMS[a]["lang"]
                ws = os.path.join(tmp, f"{t}.{a}", "ws")
                os.makedirs(ws)
                open(os.path.join(ws, A.SRC[lang]), "w").write(W.starter(lang))
                leaked = [f for f in os.listdir(ws) if "oracle" in f or f.endswith("vectors.json")]
                _check(not leaked, f"{t}/{a}: oracle data in the workspace {leaked}", problems)
                for m in cfg["models"]:
                    argv = A.claude_argv(a, t, m, cfg["max_turns"], cfg["models"][m]["per_trial_usd_cap"], stream=True)
                    _check("--model" in argv and argv[argv.index("--model") + 1] == m, f"{t}/{a}/{m}: argv has the wrong model", problems)
                    _check(argv[argv.index("--max-budget-usd") + 1] == str(cfg["models"][m]["per_trial_usd_cap"]), f"{t}/{a}/{m}: per-trial cap not passed", problems)
                    _check("stream-json" in argv, f"{t}/{a}/{m}: stream-json off (no per-turn decomposition)", problems)
        say(f"workspaces and argv: {len(cfg['tasks']) * len(cfg['arms'])} workspaces, {len(cfg['tasks']) * len(cfg['arms']) * len(cfg['models'])} argv built, oracle never copied")
        say(f"arm env: " + ", ".join(f"{a}={A.ARMS[a].get('env') or {}}" for a in cfg["arms"]))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    # guard simulation: a model whose real cost is twice the estimate is stopped by the projection, the others run
    sim = MatrixGuard(cfg, rows)
    stopped = {}
    for m, *_ in rows:
        why = sim.allow(m)
        if why:
            stopped.setdefault(m, why)
            continue
        sim.record(m, cfg["models"][m]["expected_usd_per_trial"] * (4.0 if m == list(cfg["models"])[-1] else 1.0))
    say(f"guard simulation (last model at 4x its estimate): spent ${sim.spent:.2f} <= cap ${cfg['hard_cap_usd']:.2f}; stopped: {stopped or 'none'}")
    _check(sim.spent <= cfg["hard_cap_usd"], "guard simulation overspent the cap", problems)
    # the cost decomposition, on recorded trials, priced for every model of the matrix
    if prior_trials_dir and os.path.isdir(prior_trials_dir):
        demo(cfg, prior_trials_dir, say)
    return problems


def demo(cfg, trials_dir, say=print):
    """Decompose recorded trials (pilot 6 is sonnet, stream-json): the calibration against Claude Code's own cost, and
    the same token counts priced for every model of the matrix."""
    trials = [json.load(open(os.path.join(trials_dir, f))) for f in sorted(os.listdir(trials_dir)) if f.endswith(".json")]
    say(f"cost decomposition on {len(trials)} recorded trials ({trials_dir}); thinking tokens are an estimate "
        "(output tokens minus visible text and tool inputs; pilot-6 records lack the per-turn text sizes, so it is not computed for them)")
    by_arm = {}
    for tr in trials:
        u = tr.get("usage") or {}
        split = u.get("split") or _split_from_turns(tr.get("turn_log") or [], u.get("tokens_out") or 0)
        for m in cfg["models"]:
            d = CD.decompose(split, tr.get("turn_log") or [], tr.get("final_source", ""), cfg["prices"][m], u.get("cost_usd"))
            by_arm.setdefault((tr["arm"], m), []).append(d)
    say("  arm  model   turns  lines  ctx$/turn  context$  output$  total$ (computed)  reported$  calibration")
    for (arm, m), ds in sorted(by_arm.items()):
        mean = lambda f: sum(f(d) for d in ds) / len(ds)
        ctx = mean(lambda d: d["usd"]["fresh_input"] + d["usd"]["cache_write"] + d["usd"]["cache_read"])
        outp = mean(lambda d: d["usd"]["output_visible"] + d["usd"]["output_thinking_est"])
        rep = mean(lambda d: d["usd_reported"] or 0)
        cal = rep and mean(lambda d: d["usd_total_computed"]) / rep
        say(f"  {arm:4} {m:7} {mean(lambda d: d['turns'] or 0):5.1f} {mean(lambda d: d['final_program_lines']):6.1f} "
            f"{mean(lambda d: d['usd_context_per_turn'] or 0):9.4f} {ctx:9.4f} {outp:8.4f} {mean(lambda d: d['usd_total_computed']):9.4f} "
            f"{rep:17.4f} {cal:11.2f}")
    say("  (reported$ is what the recorded model cost; calibration = computed / reported for the model the pilot used)")


def _split_from_turns(log, out_total):
    """Pilot-6 records keep the per-turn split but not the final one: sum the turns."""
    return {"fresh": sum(t.get("fresh", 0) for t in log), "cache_write": sum(t.get("cache_write", 0) for t in log),
            "cache_read": sum(t.get("cache_read", 0) for t in log), "output": out_total}
