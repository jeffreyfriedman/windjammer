"""Mutation checks on the grader: wrong solutions must fail, hard-coded sample answers must fail the hidden vectors."""
import ast
import json
import os
import random
import shutil
import tempfile
import unittest

from agentdx import baseline, tasks as T
from agentdx.paths import solution_path

FULL = os.environ.get("AGENTDX_FULL") == "1"
IDS = T.list_task_ids() if FULL else T.list_task_ids()[::6]


def hardcoded_solution(task_id):
    """A program that memorizes the visible examples and prints nothing otherwise."""
    table = [(s.get("stdin", ""), s.get("args", []), s["stdout"]) for s in T.load_samples(task_id)]
    return ("import sys\nTABLE = %r\n"
            "data = sys.stdin.read()\n"
            "for stdin, args, out in TABLE:\n"
            "    if stdin == data and args == sys.argv[1:]:\n"
            "        sys.stdout.write(out)\n"
            "        break\n") % (table,)


def run_src(task_id, src, lang="python"):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "main.py")
        open(p, "w").write(src)
        return baseline.grade_solution(lang, task_id, src=p)


class Mutants(ast.NodeTransformer):
    """Collect-and-apply single-point mutations: flip arithmetic/comparison operators, shift int constants."""
    SWAP = {ast.Add: ast.Sub, ast.Sub: ast.Add, ast.Mult: ast.FloorDiv, ast.Lt: ast.LtE, ast.LtE: ast.Lt,
            ast.Gt: ast.GtE, ast.GtE: ast.Gt, ast.Eq: ast.NotEq, ast.NotEq: ast.Eq, ast.And: ast.Or, ast.Or: ast.And}

    def __init__(self, target):
        self.target, self.n = target, 0

    def _hit(self):
        self.n += 1
        return self.n - 1 == self.target

    def visit_BinOp(self, node):
        self.generic_visit(node)
        if type(node.op) in self.SWAP and self._hit():
            node.op = self.SWAP[type(node.op)]()
        return node

    def visit_Compare(self, node):
        self.generic_visit(node)
        for i, op in enumerate(node.ops):
            if type(op) in self.SWAP and self._hit():
                node.ops[i] = self.SWAP[type(op)]()
        return node

    def visit_BoolOp(self, node):
        self.generic_visit(node)
        if type(node.op) in self.SWAP and self._hit():
            node.op = self.SWAP[type(node.op)]()
        return node

    def visit_Constant(self, node):
        if type(node.value) is int and not isinstance(node.value, bool) and self._hit():
            return ast.copy_location(ast.Constant(node.value + 1), node)
        return node


def make_mutants(src, limit, seed):
    total = Mutants(-1)
    total.visit(ast.parse(src))
    n = total.n
    idxs = list(range(n))
    random.Random(seed).shuffle(idxs)
    out = []
    for i in idxs[:limit]:
        tree = ast.parse(src)
        Mutants(i).visit(tree)
        ast.fix_missing_locations(tree)
        m = ast.unparse(tree)
        if m != ast.unparse(ast.parse(src)):
            out.append(m)
    return out


class GraderMutation(unittest.TestCase):
    def test_hardcoded_samples_fail_hidden(self):
        for t in T.list_task_ids() if FULL else IDS:
            src = hardcoded_solution(t)
            with tempfile.TemporaryDirectory() as d:
                p = os.path.join(d, "main.py")
                open(p, "w").write(src)
                from agentdx import grade
                vis = grade.grade("python", t, d, hidden=False)
                hid = grade.grade("python", t, d, hidden=True)
            self.assertEqual(vis["status"], "pass", f"{t}: memorizing program must pass the visible examples")
            self.assertEqual(hid["status"], "fail", f"{t}: memorizing program must FAIL the hidden vectors")
            self.assertLess(hid["passed"], hid["total"] // 2 + 1, t)

    def test_trivial_wrong_programs_fail(self):
        for t in IDS:
            for name, src in (("empty", ""), ("echo", "import sys\nsys.stdout.write(sys.stdin.read())\n"),
                              ("zero", "print(0)\n"), ("crash", "raise SystemExit(7)\n")):
                r = run_src(t, src)
                self.assertNotEqual(r["status"], "pass", f"{t}: trivial program '{name}' passed")

    def test_build_error_and_timeout_are_detected(self):
        t = IDS[0]
        self.assertEqual(run_src(t, "def f(:\n")["status"], "build_error")
        os.environ["AGENTDX_TIMEOUT_SCALE"] = "0.05"
        try:
            r = run_src(t, "import time\ntime.sleep(5)\n")
        finally:
            os.environ["AGENTDX_TIMEOUT_SCALE"] = "1"
        self.assertEqual(r["status"], "fail")
        self.assertIn("timeout", r["categories"])

    def test_exit_code_is_graded(self):
        # t048 expects exit status 1 for a missing file: a solution that prints right but exits 0 must fail
        t = "t048_wc_clone"
        src = open(solution_path("python", t)).read().replace("sys.exit(1)", "sys.exit(0)")
        if src == open(solution_path("python", t)).read():
            self.skipTest("solution does not use sys.exit(1) literal")
        self.assertEqual(run_src(t, src)["status"], "fail")

    def test_reference_mutants_are_killed(self):
        killed = total = 0
        survivors = []
        per_task = 8 if FULL else 5
        for t in IDS:
            src = open(solution_path("python", t)).read()
            for m in make_mutants(src, per_task, seed=hash(t) & 0xFFFF):
                total += 1
                if run_src(t, m)["status"] != "pass":
                    killed += 1
                else:
                    survivors.append(t)
        self.assertGreater(total, 10)
        score = killed / total
        print(f"\nmutation score: {killed}/{total} = {score:.2f}; survivors (possibly equivalent mutants): {sorted(set(survivors))}")
        self.assertGreaterEqual(score, 0.85)


if __name__ == "__main__":
    unittest.main()
