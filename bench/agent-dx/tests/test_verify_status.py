import unittest

from agentdx.verify_status import reference_passes, reported_status

NEW0 = "wj verify: OK, the compiled run matches the interpreted run (stdout 42 bytes, program exit status 0)\n"
NEW1 = "wj verify: OK, the compiled run matches the interpreted run (stdout 7 bytes, program exit status 1)\n"
OLD = "wj verify: OK, the compiled run matches the interpreted run (stdout 7 bytes)\n"


class VerifyStatusTest(unittest.TestCase):
    def test_new_format_status_equals_oracle(self):
        self.assertTrue(reference_passes(0, NEW0, 0))
        self.assertTrue(reference_passes(0, NEW1, 1))  # wj exits 0 although the program's status is 1

    def test_new_format_status_differs_from_oracle(self):
        self.assertFalse(reference_passes(0, NEW1, 0))
        self.assertFalse(reference_passes(0, NEW0, 1))

    def test_new_format_nonzero_wj_exit_fails(self):
        self.assertFalse(reference_passes(1, NEW1, 1))

    def test_old_format_compares_wj_exit_code(self):
        self.assertTrue(reference_passes(0, OLD, 0))
        self.assertTrue(reference_passes(1, OLD, 1))
        self.assertFalse(reference_passes(0, OLD, 1))

    def test_no_match_phrase_fails(self):
        self.assertFalse(reference_passes(0, "wj verify: MISMATCH\n", 0))

    def test_reported_status(self):
        self.assertEqual(reported_status("noise\n" + NEW1), 1)
        self.assertIsNone(reported_status(OLD))


if __name__ == "__main__":
    unittest.main()
