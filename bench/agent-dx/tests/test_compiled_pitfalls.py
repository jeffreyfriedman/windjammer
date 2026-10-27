"""Compiled-path pitfalls that the context pack warns about. Needs a built `wj` (WJ_BIN) and cargo; skipped otherwise."""
import os
import shutil
import tempfile
import unittest

from agentdx import arms
from agentdx.runners import get_runner

def build(src):
    d = tempfile.mkdtemp(prefix="pitfall-")
    open(os.path.join(d, "main.wj"), "w").write(src)
    try:
        return get_runner("wj").build(d)
    finally:
        shutil.rmtree(d, ignore_errors=True)


LEN_BAD = """fn main() {
    let v: Vec<int> = [1, 2, 3]
    let idx: int = v.len()
    println("${idx}")
}
"""
LEN_GOOD = LEN_BAD.replace("v.len()", "v.len() as int")


@unittest.skipUnless(os.environ.get("WJ_BIN") and shutil.which("cargo"), "needs WJ_BIN and cargo")
class LenToInt(unittest.TestCase):
    def test_bare_len_fails_and_cast_builds(self):
        bad, good = build(LEN_BAD), build(LEN_GOOD)
        self.assertTrue(good.ok, good.diagnostics)
        # When this starts passing, the compiler fixed len() lowering: delete the pack advice and this test.
        self.assertFalse(bad.ok)


class PackAdvice(unittest.TestCase):
    def test_pack_carries_the_annotation_advice(self):
        text = open(arms.PACK_PATH).read()
        self.assertIn("v.len() as int", text)


if __name__ == "__main__":
    unittest.main()
