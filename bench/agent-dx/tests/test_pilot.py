"""Pilot harness logic with fake agents and a fake compiler (no LLM, no cargo)."""
import json
import os
import shutil
import stat
import tempfile
import unittest
from unittest import mock

from agentdx import arms, budget, cli, friction, pilot, pilot_report, shim, stats, tasks as T
from agentdx.paths import ROOT

TASK = "t003_prime_counts"
CANARY = "CANARY-7f3a9c1e-DO-NOT-LEAK"


def fake_wj(d, exit_code=0, out="ok"):
    p = os.path.join(d, "realwj")
    open(p, "w").write(f"#!/bin/sh\necho '{out}'\nexit {exit_code}\n")
    os.chmod(p, 0o755)
    return p


def grader(compiled="pass", interp="pass"):
    g = lambda s: {"status": s, "passed": 1 if s == "pass" else 0, "total": 1, "categories": [], "failures": []}
    return lambda task, ws: {"compiled": g(compiled), "interp": g(interp)}


class FakeAgent:
    """Calls the shim like an agent would; returns a Claude-Code-shaped JSON result."""

    def __init__(self, calls, cost=0.3, wj_exit=None):
        self.calls, self.cost, self.n = calls, cost, 0

    def run(self, ws, shim_dir, arm, task):
        self.n += 1
        open(os.path.join(ws, "main.wj"), "w").write("fn main() { println(\"hi\") }\n")
        import subprocess
        for argv in self.calls:
            subprocess.run([os.path.join(shim_dir, "wj")] + argv, cwd=ws, capture_output=True)
        return {"json": {"usage": {"input_tokens": 100, "cache_creation_input_tokens": 1000, "cache_read_input_tokens": 9000,
                                   "output_tokens": 500}, "total_cost_usd": self.cost, "num_turns": 7, "duration_api_ms": 4000,
                         "subtype": "success"}, "exit": 0, "log": ""}


class TrialRecord(unittest.TestCase):
    def run_one(self, arm, calls, real_exit=0, **kw):
        with tempfile.TemporaryDirectory() as d:
            real = fake_wj(d, real_exit, kw.pop("out", "ok"))
            return pilot.run_trial(TASK, arm, FakeAgent(calls), 0, d, grader(**kw), real_wj=real)

    def test_fields_and_token_sum(self):
        tr = self.run_one("A", [["check", "--json", "main.wj"], ["run", "--interpret", "main.wj"]])
        self.assertEqual(tr["schema"], 2)
        self.assertEqual(tr["usage"]["tokens_in"], 10100)
        self.assertEqual(tr["usage"]["tokens_out"], 500)
        self.assertEqual(tr["usage"]["turns"], 7)
        self.assertEqual(tr["verify_calls"], 2)
        self.assertEqual(tr["verify_calls_to_green"], 1)
        self.assertEqual(tr["thinking_seconds"], 4.0)
        self.assertEqual(tr["mode_violations"], 0)
        self.assertTrue(tr["pass_final"])
        self.assertIn("println", tr["first_source"])

    def test_mode_violations_per_arm(self):
        self.assertEqual(self.run_one("A", [["build", "main.wj"], ["run", "--compiled", "main.wj"]])["mode_violations"], 2)
        self.assertEqual(self.run_one("B", [["run", "--interpret", "main.wj"], ["build", "main.wj"]])["mode_violations"], 1)
        self.assertEqual(self.run_one("C", [["build", "main.wj"]])["mode_violations"], 1)

    def test_failing_calls_give_no_green_and_categories(self):
        tr = self.run_one("B", [["build", "main.wj"]], real_exit=1, out="error: cannot find value `x` in scope")
        self.assertIsNone(tr["verify_calls_to_green"])
        self.assertEqual(tr["diagnostic_categories"].get("name-resolution"), 1)

    def test_green_means_start_of_final_success_run(self):
        with tempfile.TemporaryDirectory() as d:
            log = [{"argv": a, "exit": e, "secs": 1, "out": "error: x", "source": ""} for a, e in
                   [(["check"], 1), (["check"], 0), (["run", "--interpret", "m"], 1), (["run", "--interpret", "m"], 0), (["check"], 0)]]
            for e in log:
                e["verb"], e["mode"] = shim.classify(e["argv"])
            an = pilot.analyse_invocations(log)
            self.assertEqual(an["verify_calls_to_green"], 4)

    def test_divergence_flags(self):
        tr = self.run_one("A", [["run", "--interpret", "main.wj"]], compiled="fail", interp="pass")
        self.assertEqual(tr["divergence"], {"interp_pass": True, "compiled_pass": False, "diverged": True, "silent": True})
        self.assertFalse(tr["pass_final"])
        both = self.run_one("A", [], compiled="pass", interp="pass")["divergence"]
        self.assertFalse(both["silent"] or both["diverged"])
        tr = self.run_one("A", [], compiled="pass", interp="fail")
        self.assertTrue(tr["divergence"]["diverged"] and not tr["divergence"]["silent"])

    def test_time_split_sums_to_wall(self):
        tr = self.run_one("A", [["check", "main.wj"]])
        self.assertLessEqual(tr["thinking_seconds"] + tr["tool_wait_seconds"] + tr["other_seconds"], tr["wall_seconds"] + 4.01)
        self.assertGreaterEqual(tr["other_seconds"], 0)


class ArmsAndIsolation(unittest.TestCase):
    def test_arm_prompts_differ_only_in_loop_paragraph(self):
        a, b = arms.prompt(TASK, "A"), arms.prompt(TASK, "B")
        strip = lambda p, k: p.replace(arms.LOOP_TEXT[k], "")
        self.assertEqual(strip(a, "interpreted-first"), strip(b, "compiled-first"))
        self.assertIn("run --interpret", a)
        self.assertNotIn("--interpret", b.replace("Do not use `--interpret`.", ""))
        self.assertIn("wj build", b)

    def test_argv_pack_model_and_tool_restrictions(self):
        for arm in "AB":
            argv = arms.claude_argv(arm, TASK, "sonnet", 25, 2.0)
            self.assertEqual(argv[argv.index("--model") + 1], "sonnet")
            self.assertEqual(argv[argv.index("--max-turns") + 1], "25")
            self.assertEqual(argv[argv.index("--output-format") + 1], "json")
            self.assertIn("--append-system-prompt-file", argv)
            allowed = argv[argv.index("--allowedTools") + 1]
            self.assertIn("Bash(wj:*)", allowed)
            self.assertNotIn("Bash,", allowed + ",")
            self.assertEqual(argv[argv.index("--permission-mode") + 1], "dontAsk")
            self.assertEqual(argv[argv.index("--setting-sources") + 1], "project")   # no user-level CLAUDE.md/settings
            self.assertIn("--no-session-persistence", argv)
        self.assertNotIn("--append-system-prompt-file", arms.claude_argv("C", TASK, "sonnet", 25, 2.0))

    def test_canary_in_oracle_never_reaches_agent_visible_files(self):
        with tempfile.TemporaryDirectory() as d:
            tdir = os.path.join(d, "tasks")
            shutil.copytree(os.path.join(ROOT, "tasks", TASK), os.path.join(tdir, TASK))
            for n in ("ref.py", "vectors.json"):
                p = os.path.join(tdir, TASK, "oracle", n)
                open(p, "a").write("\n" + CANARY + "\n") if n == "ref.py" else None
            vec = json.load(open(os.path.join(tdir, TASK, "oracle", "vectors.json")))
            vec.append({"stdin": CANARY, "args": [], "files": {}, "stdout": CANARY})
            json.dump(vec, open(os.path.join(tdir, TASK, "oracle", "vectors.json"), "w"))
            out = os.path.join(d, "out")
            with mock.patch.object(T, "TASKS", tdir):
                self.assertIn(CANARY, json.dumps(T.load_vectors(TASK)))   # the canary really is in the oracle
                real = fake_wj(d)
                pilot.run_trial(TASK, "A", FakeAgent([["check", "main.wj"]]), 0, out, grader(), real_wj=real)
                visible = [arms.prompt(TASK, a) for a in "ABC"] + [" ".join(arms.claude_argv(a, TASK, "sonnet", 25, 2)) for a in "ABC"]
                visible.append(open(arms.PACK_PATH).read())
            ws = os.path.join(out, "work", f"{TASK}.A.0", "ws")
            for root, _, files in os.walk(ws):
                visible += [open(os.path.join(root, f), errors="ignore").read() for f in files]
            self.assertFalse([v for v in visible if CANARY in v])

    def test_canary_scan_is_not_vacuous(self):
        # control: a canary placed in the spec DOES show up in the prompt, so the scan above can fail
        with tempfile.TemporaryDirectory() as d:
            tdir = os.path.join(d, "tasks")
            shutil.copytree(os.path.join(ROOT, "tasks", TASK), os.path.join(tdir, TASK))
            open(os.path.join(tdir, TASK, "spec.md"), "a").write(CANARY)
            with mock.patch.object(T, "TASKS", tdir):
                self.assertIn(CANARY, arms.prompt(TASK, "A"))

    def test_sandbox_has_only_starter(self):
        with tempfile.TemporaryDirectory() as d:
            real = fake_wj(d)
            pilot.run_trial(TASK, "B", FakeAgent([]), 0, d, grader(), real_wj=real)
            ws = os.path.join(d, "work", f"{TASK}.B.0", "ws")
            self.assertEqual(os.listdir(ws), ["main.wj"])


class Budget(unittest.TestCase):
    def test_counter_abort(self):
        g = budget.BudgetGuard(100, 2, 0.1)
        g.before_trial(); g.record(0.1); g.before_trial(); g.record(0.1)
        with self.assertRaises(budget.BudgetExceeded):
            g.before_trial()

    def test_dollar_abort_uses_observed_mean(self):
        g = budget.BudgetGuard(1.0, 60, 0.1)
        for _ in range(3):
            g.before_trial(); g.record(0.3)       # observed 0.3/trial vs estimate 0.1
        with self.assertRaises(budget.BudgetExceeded):
            g.before_trial()                      # 0.9 + 0.3 > 1.0 (estimate alone would allow it)

    def test_ceiling_cannot_be_raised_by_flag(self):
        self.assertEqual(budget.BudgetGuard(1e9, 500, 0).max_trials, budget.PILOT_MAX_TRIALS)

    def test_run_pilot_stops_balanced_and_resumes_within_same_budget(self):
        with tempfile.TemporaryDirectory() as d:
            real = fake_wj(d)
            g = budget.BudgetGuard(100, 3, 0.3)
            r = pilot.run_pilot(FakeAgent([]), ["t003_prime_counts", "t006_base_convert"], ["A", "B"], 2, d, g, grader(), real, say=lambda *_: None)
            self.assertEqual(r["completed"], 3)
            self.assertIn("max-trials", r["aborted_reason"])
            g2 = budget.BudgetGuard(100, 3, 0.3)     # resume: the 3 recorded trials count
            r2 = pilot.run_pilot(FakeAgent([]), ["t003_prime_counts", "t006_base_convert"], ["A", "B"], 2, d, g2, grader(), real, say=lambda *_: None)
            self.assertEqual(r2["completed"], 0)
            self.assertEqual(len(pilot.load_trials(d)), 3)

    def test_plan_is_balanced_per_round(self):
        p = pilot.plan(["x", "y"], ["A", "B"], 2)
        self.assertEqual({(t, a) for t, a, k in p[:4]}, {("x", "A"), ("x", "B"), ("y", "A"), ("y", "B")})
        self.assertEqual({k for *_, k in p[:4]}, {0})

    def test_estimate_scales_with_trials_and_turns(self):
        a = budget.estimate_plan(60, 11000, 25)
        self.assertAlmostEqual(a["total_expected"], 60 * a["per_trial_expected"])
        self.assertGreater(budget.estimate_plan(60, 11000, 50)["total_worst"], a["total_worst"])
        self.assertLessEqual(a["total_expected"], a["total_worst"])

    def test_cli_requires_yes_and_refuses_oversize(self):
        with mock.patch.object(pilot, "run_pilot") as rp:
            self.assertEqual(cli.main(["pilot", "--out", "/nonexistent"]), 3)
            self.assertEqual(cli.main(["pilot", "--trials", "4", "--yes"]), 2)
            self.assertEqual(cli.main(["pilot", "--arms", "A,B,C", "--yes"]), 2)   # 90 trials
            rp.assert_not_called()
            with tempfile.TemporaryDirectory() as d:
                rp.return_value = {"completed": 60, "aborted_reason": None, "spent_usd": 1}
                self.assertEqual(cli.main(["pilot", "--yes", "--out", d]), 0)
                rp.assert_called_once()


class Stats(unittest.TestCase):
    def test_ci_contains_mean_and_constant_is_tight(self):
        m, lo, hi = stats.bootstrap_ci([[1, 0, 1], [1, 1], [0, 0, 1]], 500)
        self.assertLessEqual(lo, m); self.assertLessEqual(m, hi)
        self.assertEqual(stats.bootstrap_ci([[3, 3], [3]], 200), (3, 3, 3))
        self.assertIsNone(stats.bootstrap_ci([[None]], 10))

    def test_paired_diff_detects_shift_and_zero(self):
        a = {t: [10 + i, 11] for i, t in enumerate("abcdef")}
        b = {t: [5 + i, 6] for i, t in enumerate("abcdef")}
        pt, lo, hi = stats.paired_diff_ci(a, b, 500)
        self.assertEqual(pt, 5); self.assertGreater(lo, 0)
        c = {t: [4, 4] for t in "abc"}
        self.assertEqual(stats.paired_diff_ci(c, c, 200), (0, 0, 0))
        pt, lo, hi = stats.paired_diff_ci(a, {t: v for t, v in a.items()}, 500)   # noisy identical data: CI straddles 0
        self.assertTrue(lo < 0 < hi)


class Friction(unittest.TestCase):
    def trial(self, arm, task, samples, silent=False, secs=1):
        return {"task": task, "arm": arm, "trial": 0, "usage": {"tokens_in": 70000, "tokens_out": 3000, "turns": 10},
                "diag_samples": samples, "divergence": {"silent": silent}, "verify_calls": 6,
                "invocations": [{"verb": "build", "mode": "compiled", "secs": secs, "exit": 0}]}

    def test_rules_rank_and_locations(self):
        ts = [self.trial("A", "t1", ["error: W0003 .iter() is Rust leakage"] * 3),
              self.trial("B", "t2", ["error: W0003 .iter()", "error: cannot find value `q` in scope"]),
              self.trial("A", "t3", ["error: something novel 12"], silent=True, secs=45)]
        fr = friction.build(ts)
        ids = [f["id"] for f in fr]
        self.assertEqual(ids[0], "lint-leakage")
        self.assertEqual(fr[0]["wasted_calls"], 4)
        self.assertEqual(fr[0]["n_tasks"], 2)
        self.assertIn("lint", fr[0]["fix_kind"])
        self.assertIn("silent-divergence", ids)
        self.assertIn("slow-command:build-compiled", ids)
        self.assertTrue(any(i.startswith("other:") for i in ids))
        self.assertEqual(fr[0]["priority"], "P0")
        self.assertEqual(fr[0]["est_tokens"], 4 * 7300)

    def test_report_renders_and_lists_friction(self):
        with tempfile.TemporaryDirectory() as d:
            real = fake_wj(d, 1, "error: W0003 .iter() leakage")
            trs = [pilot.run_trial(t, a, FakeAgent([["build", "main.wj"]]), k, d, grader("fail" if a == "B" else "pass"), real_wj=real)
                   for t in ("t003_prime_counts", "t006_base_convert", "t009_rpn_eval") for a in "AB" for k in range(2)]
        md, js = pilot_report.build_report(trs, 200)
        self.assertIn("Friction list", md)
        self.assertIn("Rust-leakage lint", md)
        self.assertEqual(js["arms"]["pass"]["A"][0], 1.0)
        self.assertEqual(js["arms"]["pass"]["B"][0], 0.0)
        self.assertGreater(js["diffs"]["pass"]["A-B"][1], 0.99)


if __name__ == "__main__":
    unittest.main()
