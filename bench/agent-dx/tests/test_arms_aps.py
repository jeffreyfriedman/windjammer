"""Arms D/P/G/R, native shim, APS and delta report (fake data, no LLM)."""
import os
import tempfile
import unittest

from agentdx import aps, arms, pilot, pilot_report, shim
from tests.test_pilot import FakeAgent, fake_wj, grader

T = "t003_prime_counts"


def trial(arm, task, passed, turns=4, tin=10000, wall=30.0):
    return {"arm": arm, "task": task, "trial": 0, "pass_final": passed, "usage": {"turns": turns, "tokens_in": tin, "tokens_out": 500, "cost_usd": 0.03},
            "wall_seconds": wall, "thinking_seconds": 5, "tool_wait_seconds": 1, "verify_calls": 2, "verify_calls_to_green": 1,
            "divergence": {"silent": False, "diverged": False, "interp_pass": passed, "compiled_pass": passed}, "mode_violations": 0,
            "diagnostic_categories": {}, "invocations": [], "diag_samples": [], "lang": arms.ARMS[arm]["lang"]}


class ArmPrompts(unittest.TestCase):
    def test_d_prompt_allows_one_build_and_native_prompts_name_their_files(self):
        d = arms.prompt(T, "D")
        self.assertIn("`wj build main.wj` ONCE", d)
        self.assertIn("wj run --interpret", d)
        for arm, f, tool in (("P", "main.py", "python3"), ("G", "main.go", "go"), ("R", "main.rs", "rustc")):
            p = arms.prompt(T, arm)
            self.assertIn(f, p); self.assertIn(tool, p); self.assertNotIn("wj ", p)

    def test_policy_per_language_never_leaks_other_toolchains(self):
        self.assertIn("Bash(python3:*)", arms.tool_policy("P")["allowed"])
        self.assertNotIn("Bash(wj:*)", arms.tool_policy("P")["allowed"])
        self.assertIn("Bash(wj:*)", arms.tool_policy("D")["allowed"])
        self.assertIn("Bash(rustc:*)", arms.tool_policy("R")["allowed"])
        for a in arms.ARMS:
            self.assertNotIn("Bash", arms.tool_policy(a)["allowed"])
            argv = arms.claude_argv(a, T, "sonnet", 25, 2.0)
            self.assertEqual("--append-system-prompt-file" in argv, arms.ARMS[a]["pack"])

    def test_wj_arms_share_the_same_task_text_and_head(self):
        strip = lambda arm: arms.prompt(T, arm).replace(arms.LOOP_TEXT[arms.ARMS[arm]["loop"]], "")
        self.assertEqual(strip("A"), strip("B")); self.assertEqual(strip("A"), strip("D")); self.assertEqual(strip("A"), strip("C"))


class Violations(unittest.TestCase):
    def inv(self, *calls):
        return [{"verb": v, "mode": m, "exit": 0, "secs": 1} for v, m in calls]

    def test_d_may_build_but_not_run_compiled(self):
        self.assertEqual(pilot.violations("D", self.inv(("build", "compiled"), ("run", "interpret"))), 0)
        self.assertEqual(pilot.violations("D", self.inv(("run", "compiled"))), 1)
        self.assertEqual(pilot.violations("A", self.inv(("build", "compiled"))), 1)
        self.assertEqual(pilot.violations("P", self.inv(("native", "native"))), 0)

    def test_native_shim_classification(self):
        self.assertEqual(shim.classify(["main.py"], "python3"), ("native", "native"))


class NativeTrial(unittest.TestCase):
    def test_python_arm_trial_uses_py_file_and_shim(self):
        with tempfile.TemporaryDirectory() as d:
            class Py(FakeAgent):
                def run(self, ws, shim_dir, arm, task):
                    open(os.path.join(ws, "main.py"), "w").write("print(1)\n")
                    import subprocess
                    subprocess.run([os.path.join(shim_dir, "python3"), "main.py"], cwd=ws, capture_output=True)
                    return super().run.__func__(self, ws, shim_dir, arm, task) if False else {"json": {"usage": {}, "total_cost_usd": 0.01, "num_turns": 2}, "exit": 0, "log": ""}
            tr = pilot.run_trial(T, "P", Py([]), 0, d, grader("fail", "fail"))
            self.assertEqual(tr["lang"], "python")
            self.assertEqual(tr["verify_calls"], 1)
            self.assertIn("print(1)", tr["final_source"])
            self.assertEqual(os.listdir(os.path.join(d, "work", f"{T}.P.0", "ws")), ["main.py"])

    def test_capability_denials_counted(self):
        with tempfile.TemporaryDirectory() as d:
            real = fake_wj(d, 1, "error[WJ0040]: capability net.connect denied")
            tr = pilot.run_trial(T, "D", FakeAgent([["run", "--interpret", "main.wj"], ["build", "main.wj"]]), 0, d, grader(), real_wj=real)
            self.assertEqual(tr["capability_denials"], 2)
            self.assertEqual(tr["final_build_calls"], 1)


class APS(unittest.TestCase):
    def sets(self, lp, bp, lturns, bturns):
        tasks = [f"t{i}" for i in range(6)]
        L = [trial("A", t, i < lp, lturns) for i, t in enumerate(tasks * 1)]
        B = [trial("P", t, i < bp, bturns, tin=20000, wall=60.0) for i, t in enumerate(tasks)]
        return L, B

    def test_parity_is_100_and_faster_scores_above(self):
        L, B = self.sets(6, 6, 4, 4)
        for t in B: t["usage"]["tokens_in"] = 10000; t["wall_seconds"] = 30.0
        self.assertAlmostEqual(aps.components(L, B)["APS"], 100.0)
        L2, B2 = self.sets(6, 6, 2, 4)
        self.assertGreater(aps.components(L2, B2)["APS"], 100)

    def test_weights_exact(self):
        L, B = self.sets(6, 4, 4, 4)       # P = 1.5, I = T = W = 1 over passing trials
        for t in B: t["usage"]["tokens_in"] = 10000; t["wall_seconds"] = 30.0
        self.assertAlmostEqual(aps.components(L, B)["APS"], 100 * 1.5 ** 0.4)

    def test_p_limited_and_clip(self):
        L, B = self.sets(3, 6, 4, 4)
        c = aps.components(L, B)
        self.assertIsNone(c["APS"]); self.assertAlmostEqual(c["P"], 0.5)
        L, B = self.sets(6, 6, 1, 40)
        self.assertEqual(aps.components(L, B)["I"], 4.0)

    def test_failing_trials_do_not_earn_speed_credit(self):
        L, B = self.sets(5, 6, 4, 4)
        L[5]["usage"]["turns"] = 1; L[5]["wall_seconds"] = 1   # the failing trial is very fast: must not matter
        a = aps.components(L, B)
        L[5]["usage"]["turns"] = 25
        self.assertEqual(a["I"], aps.components(L, B)["I"])

    def test_bootstrap_ci_brackets_point(self):
        L, B = self.sets(6, 6, 2, 4)
        r = aps.aps(L, B, 200)
        lo, hi = r["ci"]["I"]
        self.assertLessEqual(lo, r["point"]["I"] + 1e-9); self.assertGreaterEqual(hi, r["point"]["I"] - 1e-9)

    def test_report_has_aps_table_and_deltas(self):
        cur = [trial(a, f"t{i}", True) for a in "ADP" for i in range(4)]
        prev = [trial("A", f"t{i}", False) for i in range(4)]
        md, js = pilot_report.build_report(cur, 100, prev=prev)
        self.assertIn("Agent preference score", md)
        self.assertIn("Deltas versus the previous pilot", md)
        self.assertGreater(js["deltas"]["A"]["pass"], 0.99)
        self.assertIn("A-vs-P", js["aps"])


if __name__ == "__main__":
    unittest.main()
