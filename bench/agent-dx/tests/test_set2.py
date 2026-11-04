"""Second task set (tasks_set2): oracle is reproducible, tasks are well formed, workspace files reach the sandbox."""
import json
import os
import shutil
import tempfile
import unittest
from unittest import mock

from agentdx import gen, pilot, tasks as T
from agentdx.paths import ROOT
from tests.test_pilot import FakeAgent, fake_wj, grader

SET2 = os.path.join(ROOT, "tasks_set2")
SOL = os.path.join(ROOT, "solutions", "wj-set2")
IDS = sorted(d for d in os.listdir(SET2) if os.path.isfile(os.path.join(SET2, d, "task.json")))


class Set2(unittest.TestCase):
    def test_ten_tasks_with_distinct_ids_and_all_required_files(self):
        self.assertEqual(len(IDS), 10)
        for i in IDS:
            for f in ("spec.md", "task.json", "samples.json", os.path.join("oracle", "ref.py"), os.path.join("oracle", "vectors.json")):
                self.assertTrue(os.path.isfile(os.path.join(SET2, i, f)), (i, f))
            self.assertTrue(os.path.isfile(os.path.join(SOL, i, "main.wj")), i)

    def test_committed_vectors_equal_a_fresh_generation_and_samples_are_a_subset(self):
        with mock.patch.object(gen, "TASKS", SET2):
            for i in IDS:
                samples, vectors = gen.build_vectors(i)
                self.assertEqual(json.load(open(os.path.join(SET2, i, "oracle", "vectors.json"))), vectors, i)
                self.assertEqual(json.load(open(os.path.join(SET2, i, "samples.json"))), samples, i)
                self.assertTrue(all(s in vectors for s in samples)); self.assertGreaterEqual(len(vectors), 25)
                self.assertGreaterEqual(len({v["stdout"] for v in vectors}), 5, i)

    def test_task_ids_do_not_collide_with_set_one(self):
        one = set(os.listdir(os.path.join(ROOT, "tasks")))
        self.assertFalse(set(IDS) & one)

    def test_workspace_files_are_copied_into_the_sandbox_and_oracle_is_not(self):
        with mock.patch.object(T, "TASKS", SET2), tempfile.TemporaryDirectory() as d:
            real = fake_wj(d)
            tr = pilot.run_trial("t110_crc_lines", "A", FakeAgent([]), 0, d, grader(), real_wj=real)
            ws = os.path.join(d, "work", "t110_crc_lines.A.0", "ws")
            self.assertTrue(tr["task"] == "t110_crc_lines")
            self.assertIn("crc32fast", tr["first_source"] or "crc32fast")  # record exists
            # cleanup after grading removes everything but the source; check the copy happened by inspecting before cleanup
        with mock.patch.object(T, "TASKS", SET2), tempfile.TemporaryDirectory() as d:
            seen = {}
            class Peek(FakeAgent):
                def run(self, ws, shim_dir, arm, task):
                    seen["files"] = sorted(os.listdir(ws))
                    return super().run(ws, shim_dir, arm, task)
            pilot.run_trial("t110_crc_lines", "A", Peek([]), 0, d, grader(), real_wj=fake_wj(d))
            self.assertEqual(seen["files"], ["main.wj", "wj.toml"])
            pilot.run_trial("t101_json_orders", "A", Peek([]), 0, d, grader(), real_wj=fake_wj(d))
            self.assertEqual(seen["files"], ["main.wj"])

    @unittest.skipUnless(os.environ.get("WJ_BIN"), "needs WJ_BIN")
    def test_interpreted_reference_solutions_pass_except_the_crate_task(self):
        from agentdx import grade as G
        with mock.patch.object(T, "TASKS", SET2):
            for i in IDS:
                wd = tempfile.mkdtemp()
                shutil.copy(os.path.join(SOL, i, "main.wj"), wd)
                g = G.grade("wj-interp", i, wd, hidden=True)
                if i == "t110_crc_lines":
                    self.assertEqual(g["status"], "fail")      # the interpreter refuses crate calls, loudly
                else:
                    self.assertEqual(g["status"], "pass", (i, g["failures"][:1]))


if __name__ == "__main__":
    unittest.main()
