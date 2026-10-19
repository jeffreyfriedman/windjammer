"""Guardrails for the context pack: no task-specific hints; advice about stdin is anchored to graded reference solutions."""
import glob
import os
import re
import unittest

from agentdx import arms, tasks as T
from agentdx.paths import SOLUTIONS


def leaks(text):
    low = text.lower()
    out = []
    for tid in T.list_task_ids():
        title = T.load_task(tid)["title"].strip().lower()
        if tid.lower() in low or (len(title) >= 8 and title in low):
            out.append(tid)
    return out


class PackGuardrails(unittest.TestCase):
    def setUp(self):
        self.pack = open(arms.PACK_PATH).read()

    def test_pack_has_no_task_ids_or_titles(self):
        self.assertEqual(leaks(self.pack), [])

    def test_leak_detector_is_not_vacuous(self):
        title = T.load_task("t011_lru_cache")["title"]
        self.assertEqual(leaks(self.pack + "\n" + title), ["t011_lru_cache"])
        self.assertEqual(leaks(self.pack + " t050_cat_number"), ["t050_cat_number"])

    def test_stdin_advice_is_anchored_to_reference_solutions(self):
        self.assertIn('fs::read_to_string("/dev/stdin")', self.pack)
        users = [p for p in glob.glob(os.path.join(SOLUTIONS, "wj", "*", "main.wj"))
                 if 'fs::read_to_string("/dev/stdin")' in open(p).read()]
        self.assertGreaterEqual(len(users), 20)   # the graded baseline (59/60 pass) exercises the recommended spelling

    def test_pack_does_not_recommend_the_nonterminating_form(self):
        for m in re.finditer(r"read_line", self.pack):
            ctx = self.pack[max(0, m.start() - 80):m.end() + 80]
            self.assertTrue("loops forever" in ctx or "not implemented" in ctx, ctx)


if __name__ == "__main__":
    unittest.main()
