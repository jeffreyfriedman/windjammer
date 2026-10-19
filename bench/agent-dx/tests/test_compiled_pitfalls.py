"""Compiled-path pitfalls that the context pack warns about. Needs a built `wj` (WJ_BIN) and cargo; skipped otherwise."""
import os
import shutil
import tempfile
import unittest

from agentdx import arms
from agentdx.runners import get_runner

BAD = """fn main() {
    let a: int = 3
    let mut v = 0
    v = v * a + 1
    while v > 0 {
        v = v / a
    }
    println("${v}")
}
"""
GOOD = BAD.replace("let mut v = 0", "let mut v: int = 0")


def build(src):
    d = tempfile.mkdtemp(prefix="pitfall-")
    open(os.path.join(d, "main.wj"), "w").write(src)
    try:
        return get_runner("wj").build(d)
    finally:
        shutil.rmtree(d, ignore_errors=True)


@unittest.skipUnless(os.environ.get("WJ_BIN") and shutil.which("cargo"), "needs WJ_BIN and cargo")
class UntypedIntAccumulator(unittest.TestCase):
    def test_unannotated_fails_and_annotated_builds(self):
        bad, good = build(BAD), build(GOOD)
        self.assertTrue(good.ok, good.diagnostics)
        # When this starts passing, the compiler fixed literal defaulting: delete the pack advice and this test.
        self.assertFalse(bad.ok)
        self.assertTrue(any("mismatched types" in d["message"] for d in bad.diagnostics))


class PackAdvice(unittest.TestCase):
    def test_pack_carries_the_annotation_advice(self):
        text = open(arms.PACK_PATH).read()
        self.assertIn("let mut v: int = 0", text)


if __name__ == "__main__":
    unittest.main()
