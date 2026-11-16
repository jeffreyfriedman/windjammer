import json
import os
import tempfile
import unittest

from agentdx import preflight as PF

NAMES = [f"t30{i}_x" for i in range(1, 10)]
OK = "wj verify: OK, the compiled run matches the interpreted run (stdout 3 bytes, program exit status {})\n"
OLD = "wj verify: OK, the compiled run matches the interpreted run (stdout 3 bytes)\n"


def make_tree(root, codes=None):
    codes = codes or {}
    for n in NAMES:
        os.makedirs(os.path.join(root, "tasks", n, "oracle"))
        os.makedirs(os.path.join(root, "sol", n))
        open(os.path.join(root, "sol", n, "main.wj"), "w").write("fn main() {}")
        vecs = [{"stdin": "a", "stdout": "short"}, {"stdin": "abc", "stdout": "out\n", "code": codes.get(n, 0)}]
        json.dump(vecs, open(os.path.join(root, "tasks", n, "oracle", "vectors.json"), "w"))
    return os.path.join(root, "tasks"), os.path.join(root, "sol")


def runner_for(status=0, rc=0, out="out\n", err=None, per=None):
    def run(wj, ws, stdin):
        assert stdin == "abc"  # richest vector chosen
        s, r, o, e = (per or {}).get(os.path.basename(ws), (status, rc, out, err))
        return r, o, (e if e is not None else OK.format(s))
    return run


class ReferencesGateTest(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory()
        self.addCleanup(self.d.cleanup)

    def check(self, tasks, sol, runner, **kw):
        return PF.check_references("wj", tasks, sol, runner=runner, **kw)

    def test_pass(self):
        t, s = make_tree(self.d.name)
        self.assertEqual(self.check(t, s, runner_for()), [])

    def test_nonzero_oracle_code_passes_with_matching_status(self):
        t, s = make_tree(self.d.name, {"t303_x": 2})
        self.assertEqual(self.check(t, s, runner_for(per={"t303_x": (2, 0, "out\n", None)})), [])

    def test_wrong_exit_status_fails(self):
        t, s = make_tree(self.d.name, {"t303_x": 2})
        red = self.check(t, s, runner_for(status=0))  # programs say 0, oracle wants 2 for t303
        self.assertEqual(len(red), 1)
        self.assertIn("t303_x", red[0])

    def test_stdout_mismatch_fails(self):
        t, s = make_tree(self.d.name)
        red = self.check(t, s, runner_for(per={"t305_x": (0, 0, "different\n", None)}))
        self.assertEqual(len(red), 1)
        self.assertIn("stdout differs", red[0])

    def test_compiled_run_mismatch_fails(self):
        t, s = make_tree(self.d.name)
        red = self.check(t, s, runner_for(per={"t301_x": (0, 0, "out\n", "wj verify: MISMATCH\n")}))
        self.assertEqual(len(red), 1)

    def test_old_format_falls_back_to_exit_code(self):
        t, s = make_tree(self.d.name, {"t302_x": 1})
        self.assertEqual(self.check(t, s, runner_for(per={"t302_x": (0, 1, "out\n", OLD)})), [])
        self.assertEqual(len(self.check(t, s, runner_for(per={"t302_x": (0, 0, "out\n", OLD)}))), 1)

    def test_missing_dirs_fail_cleanly(self):
        t, s = make_tree(self.d.name)
        missing = os.path.join(self.d.name, "nope")
        self.assertIn("does not exist", self.check(missing, s, runner_for())[0])
        self.assertIn("does not exist", self.check(t, missing, runner_for())[0])

    def test_missing_oracle_and_solution_fail_cleanly(self):
        t, s = make_tree(self.d.name)
        os.remove(os.path.join(t, "t304_x", "oracle", "vectors.json"))
        os.remove(os.path.join(s, "t306_x", "main.wj"))
        self.assertEqual(len(self.check(t, s, runner_for())), 2)

    def test_oracle_dir_override(self):
        t, s = make_tree(self.d.name)
        self.assertEqual(self.check(t, s, runner_for(), oracle_dir=t), [])
        self.assertIn("oracle", self.check(t, s, runner_for(), oracle_dir=os.path.join(self.d.name, "none"))[0])

    def test_wrong_reference_count_fails(self):
        t, s = make_tree(self.d.name)
        self.assertIn("expected 9", self.check(t, s, runner_for(), pattern="t30[1-8]_*")[0])

    def test_missing_wj_fails_cleanly_with_real_runner(self):
        t, s = make_tree(self.d.name)
        self.assertIn("wj binary", PF.check_references(os.path.join(self.d.name, "no-wj"), t, s)[0])

    def test_run_references_exit_codes(self):
        t, s = make_tree(self.d.name)

        class A:
            wj, tasks_dir, solutions_dir, oracle_dir = "wj", t, s, None
        lines = []
        self.assertEqual(PF.run_references(A, runner_for(), lines.append), 0)
        self.assertEqual(PF.run_references(A, runner_for(status=1), lines.append), 1)
        self.assertTrue(any(l.startswith("RED   references") for l in lines))


if __name__ == "__main__":
    unittest.main()
