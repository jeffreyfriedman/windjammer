"""Agent-in-the-loop driver, metrics and deterministic replay, exercised with scripted fake agents (no LLM)."""
import json
import os
import sys
import tempfile
import unittest

from agentdx import driver, metrics, replay, report, workspace
from agentdx.paths import ROOT

TASK = "t002_fizzbuzz_custom"
FAKE = os.path.join(ROOT, "tests", "fake_agent.py")


def agent(mode):
    return driver.ShellAgent(f"{sys.executable} {FAKE} --mode {mode} --task {{task}} --iteration {{iteration}} --workspace {{workspace}}")


class DriverTests(unittest.TestCase):
    def test_green_first_try(self):
        tr = driver.run_trial(TASK, "python", agent("good"), max_iters=3)
        m = tr["metrics"]
        self.assertTrue(m["pass_at_1"])
        self.assertEqual(m["iterations_to_green"], 1)
        self.assertEqual(len(tr["iterations"]), 1)
        self.assertIsNone(m["first_error_to_fix"])
        self.assertEqual(m["tokens_in"], 1000)
        self.assertEqual(m["tokens_out"], 100)

    def test_wrong_then_good(self):
        tr = driver.run_trial(TASK, "python", agent("wrong_then_good"), max_iters=3)
        m = tr["metrics"]
        self.assertFalse(m["pass_at_1"])
        self.assertEqual(m["iterations_to_green"], 2)
        self.assertIn("wrong-output", m["diagnostic_categories"])
        self.assertEqual(m["tokens_in"], 3000)  # 1000 + 2000, summed over iterations

    def test_syntax_error_then_fix_records_distance_and_category(self):
        tr = driver.run_trial(TASK, "python", agent("syntax_then_good"), max_iters=3)
        m = tr["metrics"]
        self.assertEqual(m["iterations_to_green"], 2)
        self.assertEqual(m["diagnostic_categories"].get("syntax"), 1)
        self.assertEqual(m["first_error_to_fix"]["attempts"], 1)
        self.assertGreater(m["first_error_to_fix"]["changed_lines"], 0)

    def test_never_green_within_budget(self):
        tr = driver.run_trial(TASK, "python", driver.ShellAgent("echo 'print(1)' > main.py"), max_iters=2)
        self.assertFalse(tr["metrics"]["green"])
        self.assertEqual(len(tr["iterations"]), 2)
        self.assertIsNone(tr["metrics"]["iterations_to_green"])
        self.assertIsNone(tr["metrics"]["tokens_in"])  # not reported -> null, not 0

    def test_in_turn_check_calls_are_logged(self):
        tr = driver.run_trial(TASK, "python", agent("checking"), max_iters=2)
        checks = tr["iterations"][0]["agent_checks"]
        self.assertEqual([c["build_ok"] for c in checks], [False, True])
        self.assertEqual(tr["metrics"]["first_error_to_fix"]["attempts"], 1)

    def test_workspace_never_contains_oracle(self):
        with tempfile.TemporaryDirectory() as d:
            workspace.make(TASK, "python", os.path.join(d, "ws"), os.path.join(d, "log"))
            names = []
            for root, _, files in os.walk(d):
                names += [os.path.join(root, f) for f in files]
            blob = " ".join(names)
            self.assertNotIn("vectors", blob)
            self.assertNotIn("oracle", blob)
            prompt = open(os.path.join(d, "ws", "PROMPT.md")).read()
            hidden = json.load(open(os.path.join(ROOT, "tasks", TASK, "oracle", "vectors.json")))
            samples = json.load(open(os.path.join(ROOT, "tasks", TASK, "samples.json")))
            extra = [v for v in hidden if v not in samples]
            self.assertTrue(extra)
            self.assertNotIn(extra[-1]["stdout"].strip(), prompt.replace("\n", "") if len(extra[-1]["stdout"]) > 40 else "\x00")

    def test_replay_is_deterministic_and_detects_tampering(self):
        tr = driver.run_trial(TASK, "python", agent("syntax_then_good"), max_iters=3)
        with tempfile.TemporaryDirectory() as d:
            p = driver.save(tr, d)
            _, diffs = replay.replay(replay.load(p))
            self.assertEqual(diffs, [])
            t2 = replay.load(p)
            t2["iterations"][1]["hidden"]["status"] = "fail"
            _, diffs = replay.replay(t2)
            self.assertTrue(diffs)
            # recorded-only path needs no toolchain at all
            self.assertTrue(replay.recorded_metrics(replay.load(p))["green"])

    def test_report_from_transcripts(self):
        trs = [driver.run_trial(TASK, "python", agent(m), max_iters=3) for m in ("good", "wrong_then_good")]
        md, js = report.build_report(None, trs)
        self.assertIn("pass@1", md)
        a = js["agent_aggregates"]["python"]
        self.assertEqual(a["trials"], 2)
        self.assertEqual(a["pass_at_1"], 0.5)
        self.assertEqual(a["pass_within_iterations"]["2"], 1.0)


class CliSmoke(unittest.TestCase):
    def test_run_replay_report_roundtrip(self):
        from agentdx import cli
        with tempfile.TemporaryDirectory() as d:
            rc = cli.main(["run", "--langs", "python", "--tasks", "t001", "--max-iters", "2", "--trials", "2", "--out", d,
                           "--agent-cmd", "sh {harness}/agents/replay_example.sh {workspace} {task} {lang}"])
            self.assertEqual(rc, 0)
            self.assertEqual(len(os.listdir(d)), 2)
            self.assertEqual(cli.main(["replay", "--transcripts", d]), 0)
            self.assertEqual(cli.main(["replay", "--transcripts", d, "--no-regrade"]), 0)
            md = os.path.join(d, "r.md")
            cli.main(["report", "--transcripts", d, "--md", md])
            self.assertIn("python", open(md).read())


class MetricTests(unittest.TestCase):
    def test_pass_at_k_known_values(self):
        self.assertAlmostEqual(metrics.pass_at_k(10, 0, 1), 0.0)
        self.assertAlmostEqual(metrics.pass_at_k(10, 10, 1), 1.0)
        self.assertAlmostEqual(metrics.pass_at_k(10, 5, 1), 0.5)
        self.assertAlmostEqual(metrics.pass_at_k(4, 1, 2), 0.5)  # 1 - C(3,2)/C(4,2) = 1 - 3/6
        self.assertAlmostEqual(metrics.pass_at_k(5, 1, 5), 1.0)

    def test_changed_lines(self):
        self.assertEqual(metrics.changed_lines("a\nb\nc", "a\nB\nc"), 1)
        self.assertEqual(metrics.changed_lines("a", "a"), 0)
        self.assertEqual(metrics.changed_lines("", "x\ny"), 2)


if __name__ == "__main__":
    unittest.main()
