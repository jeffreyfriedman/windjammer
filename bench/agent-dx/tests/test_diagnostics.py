import unittest

from agentdx import compare, diagnostics as dg


class Diagnostics(unittest.TestCase):
    def test_rust_codes(self):
        self.assertEqual(dg.categorize("rust", "E0382 borrow of moved value"), "ownership-borrow")
        self.assertEqual(dg.categorize("rust", "E0308 mismatched types"), "type-mismatch")
        self.assertEqual(dg.categorize("rust", "E0425 cannot find value"), "name-resolution")

    def test_python_go_ts(self):
        self.assertEqual(dg.categorize("python", "SyntaxError: invalid syntax"), "syntax")
        self.assertEqual(dg.categorize("go", "./main.go:3:2: undefined: foo"), "name-resolution")
        self.assertEqual(dg.categorize("ts", "error TS2322: Type 'string' is not assignable"), "type-mismatch")

    def test_wj(self):
        self.assertEqual(dg.categorize("wj", "error: explicit .unwrap() call [W0002]"), "lint-style")
        self.assertEqual(dg.categorize("wj-interp", "unsupported in the interpreter: spawn"), "unsupported")

    def test_rust_json_parser(self):
        line = '{"level":"error","message":"mismatched types","code":{"code":"E0308"}}'
        self.assertEqual(dg.parse_rust_json(line)[0]["category"], "type-mismatch")


class Compare(unittest.TestCase):
    def test_normalization(self):
        self.assertTrue(compare.same("a \nb\n\n", "a\nb"))
        self.assertTrue(compare.same("a\r\nb", "a\nb\n"))
        self.assertFalse(compare.same("a\nb", "a\n b"))
        self.assertFalse(compare.same("a\n\nb", "a\nb"))


if __name__ == "__main__":
    unittest.main()
