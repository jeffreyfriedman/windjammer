"""The oracle must be right: independent reference implementations in different languages agree with it,
vectors are reproducible, and visible samples are a subset of the hidden vectors."""
import json
import os
import unittest

from agentdx import baseline, gen, tasks as T
from agentdx.paths import TASKS

IDS = [t for t in T.list_task_ids() if not os.environ.get("AGENTDX_TASKS") or t.startswith(tuple(os.environ["AGENTDX_TASKS"].split(",")))]
# python + go solutions were written from the spec text only (never from the oracle code).
INDEPENDENT_REFS = os.environ.get("AGENTDX_ORACLE_LANGS", "python,go").split(",")


class OracleStructure(unittest.TestCase):
    def test_suite_size(self):
        self.assertGreaterEqual(len(T.list_task_ids()), 60)

    def test_vectors_reproducible(self):
        for t in IDS:
            samples, vectors = gen.build_vectors(t)
            self.assertEqual(vectors, T.load_vectors(t), f"{t}: committed vectors differ from regenerated (seeded) ones")
            self.assertEqual(samples, T.load_samples(t))

    def test_samples_inside_vectors_and_enough_cases(self):
        for t in IDS:
            v, s = T.load_vectors(t), T.load_samples(t)
            for x in s:
                self.assertIn(x, v, t)
            self.assertGreaterEqual(len(v), 25, t)
            self.assertGreater(len(v), len(s) + 15, f"{t}: hidden set must be much larger than the visible samples")
            self.assertGreaterEqual(len({x["stdout"] for x in v}), 5, f"{t}: outputs not diverse")


class OracleCrossLanguage(unittest.TestCase):
    pass


def _mk(lang, tid):
    def f(self):
        r = baseline.grade_solution(lang, tid)
        self.assertEqual(r["status"], "pass", f"{lang} reference disagrees with the oracle on {tid}: {r['failures'][:1]} {r['build']['diagnostics'][:1]}")
    return f


for _l in INDEPENDENT_REFS:
    for _t in IDS:
        setattr(OracleCrossLanguage, f"test_{_l}_{_t}", _mk(_l, _t))

if __name__ == "__main__":
    unittest.main()
