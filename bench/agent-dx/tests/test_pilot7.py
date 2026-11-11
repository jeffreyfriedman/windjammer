"""Pilot 7 harness (no LLM): the matrix guard, the cost decomposition, the stream fields it needs, and the dry run."""
import json
import os
import unittest

from agentdx import costdecomp as CD, pilot7 as P7, stream as ST


def cfg(tasks=("t101_json_orders",), tasks_dir="tasks_set2"):
    c = P7.load_config()
    c = json.loads(json.dumps(c))
    c["tasks"], c["tasks_dir"] = list(tasks), tasks_dir
    return c


class GuardTest(unittest.TestCase):
    def test_expensive_model_is_stopped_by_the_projection_and_others_finish(self):
        c = cfg(tasks=[f"t{i}" for i in range(10)])
        rows = P7.matrix(c)
        g = P7.MatrixGuard(c, rows)
        ran = {m: 0 for m in c["models"]}
        for m, *_ in rows:
            if g.allow(m):
                continue
            ran[m] += 1
            g.record(m, c["models"][m]["expected_usd_per_trial"] * (10 if m == "opus" else 1))
        self.assertEqual(ran["haiku"], 60)
        self.assertEqual(ran["sonnet"], 60)
        self.assertLess(ran["opus"], 60)
        self.assertIn("opus", g.aborted)
        self.assertLessEqual(g.spent, c["hard_cap_usd"])

    def test_within_estimate_runs_everything(self):
        c = cfg(tasks=[f"t{i}" for i in range(10)])
        rows = P7.matrix(c)
        g = P7.MatrixGuard(c, rows)
        for m, *_ in rows:
            self.assertIsNone(g.allow(m))
            g.record(m, c["models"][m]["expected_usd_per_trial"])
        self.assertEqual(sum(g.done.values()), 180)


class DecompositionTest(unittest.TestCase):
    PRICE = {"input": 4.0, "cache_write": 5.0, "cache_read": 0.2, "output": 20.0}

    def test_dollars_are_tokens_times_prices_and_thinking_is_the_invisible_output(self):
        turns = [{"text_chars": 350, "tool_input_chars": 3500, "write_chars": 3000, "result_chars": 700}]
        d = CD.decompose({"fresh": 1000, "cache_write": 2000, "cache_read": 100000, "output": 3000}, turns, "a\n\nb\n", self.PRICE, 0.1)
        self.assertEqual(d["tokens"]["output_visible_est"], 1100)
        self.assertEqual(d["tokens"]["thinking_est"], 1900)
        self.assertAlmostEqual(d["usd"]["cache_read"], 0.02)
        self.assertAlmostEqual(d["usd_total_computed"], 0.004 + 0.01 + 0.02 + 0.022 + 0.038)
        self.assertEqual(d["final_program_lines"], 2)
        self.assertEqual(d["tokens"]["final_write_calls_est"], 857)

    def test_attribution_sums_to_the_total_difference(self):
        a = [CD.decompose({"fresh": 0, "cache_write": 0, "cache_read": 10000, "output": 1000}, [{"text_chars": 0, "tool_input_chars": 0}], "", self.PRICE)]
        b = [CD.decompose({"fresh": 0, "cache_write": 0, "cache_read": 30000, "output": 2000}, [{"text_chars": 0, "tool_input_chars": 0}] * 2, "", self.PRICE)]
        diff = CD.attribute_difference(a, b, self.PRICE)
        self.assertAlmostEqual(diff["context_usd"] + diff["visible_output_usd"] + diff["thinking_usd_est"], diff["usd_total"])
        self.assertEqual(diff["turns"], 1)


class StreamFieldsTest(unittest.TestCase):
    def test_turn_log_carries_text_thinking_tool_and_write_sizes(self):
        ev = [{"type": "assistant", "message": {"id": "m1", "usage": {"input_tokens": 1, "output_tokens": 50},
               "content": [{"type": "thinking", "thinking": ""}, {"type": "text", "text": "hello"},
                           {"type": "tool_use", "name": "Write", "input": {"file_path": "main.wj", "content": "fn main() {}\n"}}]}},
              {"type": "result", "total_cost_usd": 0.01}]
        _, log, _ = ST.parse("\n".join(json.dumps(e) for e in ev))
        self.assertEqual(log[0]["text_chars"], 5)
        self.assertEqual(log[0]["write_chars"], 13)
        self.assertGreater(log[0]["tool_input_chars"], 13)


class DryRunTest(unittest.TestCase):
    def test_dry_run_builds_every_workspace_and_argv_without_a_model(self):
        c = cfg(tasks=["t101_json_orders", "t103_trait_pricing", "t104_activity_log", "t105_date_diff", "t106_priority_queue",
                       "t107_text_wrap", "t108_to_rpn", "t109_spiral"])
        out = []
        problems = [p for p in P7.dry_run(c, None, say=out.append) if "AGENTDX_RUSTC" not in p]
        self.assertEqual(problems, [], problems)
        self.assertTrue(any("144 trials" in l for l in out), out)

    def test_dry_run_reports_a_missing_task(self):
        problems = P7.dry_run(cfg(tasks=["t999_nope"]), None, say=lambda *_: None)
        self.assertTrue(any("t999_nope" in p for p in problems))


class RelativeWorkdirTest(unittest.TestCase):
    def test_build_dir_is_absolute_for_a_relative_workdir(self):
        from agentdx import runners
        import tempfile
        here = os.getcwd()
        with tempfile.TemporaryDirectory() as d:
            os.chdir(d)
            try:
                os.makedirs("rel/ws")
                self.assertTrue(os.path.isabs(runners.RustRunner().build_dir("rel/ws")))
            finally:
                os.chdir(here)
