"""Pilot 11 denial shapes, replayed offline (no claude, no model). Each case of fixtures/denials/probe_cases.json is a neutral copy of a
denied command line from pilot 11 or of its recommended replacement, with the decision MEASURED on Claude Code 2.1.331 by
`python3 -m agentdx permission-probe` (a fake local model, no paid call). predict() must agree with every recorded decision, the prompts
must carry the replacement rule, and the input-file cost metric must count."""
import json
import os
import re
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from agentdx import arms as A, denials as DN, permprobe as PP, tasks as T  # noqa: E402

CASES = PP.load_cases()


class Predictor(unittest.TestCase):
    def test_offline_matcher_agrees_with_every_recorded_claude_code_decision(self):
        for c in CASES:
            got = "denied" if PP.predict(c["command"], c["arm"]) else "allowed"
            self.assertEqual(got, c["expect"], f"{c['arm']} {c['command']!r}: {PP.predict(c['command'], c['arm'])}")

    def test_every_pilot11_denied_shape_has_a_replacement_that_is_allowed_for_the_same_arm(self):
        for arm in ("R", "U7", "V3"):
            self.assertTrue(any(c["arm"] == arm and c["expect"] == "denied" for c in CASES), arm)
            self.assertTrue(any(c["arm"] == arm and c["expect"] == "allowed" and "replacement" in c["shape"] for c in CASES), arm)

    def test_the_still_forbidden_forms_stay_denied(self):
        for cmd in ("printf 'set A1 =A2\\n' | ./main", "echo hi > f.txt", "wj run --compiled main.wj > o.txt", "echo $(date)", "x=1 echo hi", "cat in.txt | tee o.txt", "pkill main",
                    "cd /tmp && ls", "cat /etc/passwd | wj run main.wj", "timeout 5 sleep 9"):
            self.assertTrue(PP.predict(cmd, "U7"), cmd)

    def test_replacement_forms_leave_no_denial(self):
        for arm, cmd in (("R", "printf 'set A1 \\075A2*3\\n' | ./main"), ("R", "cat in.txt | ./main"), ("U7", "wj run --compiled --stdin-file in.txt main.wj"),
                         ("U7", "printf 'a\\n' | wj run --compiled main.wj"), ("V3", "printf 'x \\075y\\n' | wj run --verify main.wj"),
                         ("V3", "wj run --verify --stdin-file in.txt main.wj")):
            self.assertEqual(PP.predict(cmd, arm), [], cmd)

    def test_octal_escape_hides_the_character_from_the_shape_marks_but_not_the_program(self):
        raw = "printf 'x \\075y\\n'"
        self.assertNotIn("space-eq", DN.shape(raw))
        self.assertIn("space-eq", DN.shape("printf 'x =y\\n'"))

    def test_mutation_prediction_changes_when_the_allow_set_loses_a_helper(self):
        orig = A.SHELL_HELPERS
        try:
            A.SHELL_HELPERS = [h for h in orig if h != "cat"]
            self.assertTrue(PP.predict("cat in.txt | ./main", "R"))
        finally:
            A.SHELL_HELPERS = orig
        self.assertEqual(PP.predict("cat in.txt | ./main", "R"), [])


class Prompts(unittest.TestCase):
    def test_every_arm_prompt_states_the_replacement_rule_for_its_own_toolchain(self):
        for arm in ("U7", "V3", "R", "P", "G"):
            h = A.head(A.ARMS[arm]["lang"])
            self.assertIn("`=` is `\\075`", h, arm)
            self.assertIn("cat in.txt | ", h) if arm != "U7" and arm != "V3" else self.assertIn("--stdin-file in.txt", h)
            self.assertIn("never `cd` elsewhere", h, arm)
            self.assertIn("no `timeout` or `pkill`", h, arm)
        self.assertIn("never bare `wj`", A.head("wj"))
        self.assertNotIn("bare `wj`", A.head("rust"))

    def test_wj_loops_show_the_input_file_form_and_no_loop_shows_a_forbidden_form(self):
        self.assertIn("--stdin-file in.txt", A.LOOP_TEXT["compiled-first"])
        self.assertIn("--stdin-file in.txt", A.LOOP_TEXT["verify-one-command"])
        for k, t in A.LOOP_TEXT.items():
            for code in re.findall(r"`([^`]+)`", t):
                if re.match(r"(wj (build|run|test|check)\b|printf|rustc|go |python3)", code) and "&&" not in code:
                    arm = "R" if code.startswith("rustc") else ("G" if code.startswith("go") else ("P" if code.startswith("python3") else "U7"))
                    self.assertEqual(PP.predict(code.replace("<input>", "a").replace("\\n", "\\n"), arm), [], f"{k}: {code}")

    def test_task_note_for_space_equals_input_is_in_every_arm_with_its_own_route(self):
        root = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tasks_set3")
        old = T.TASKS
        T.TASKS = root
        try:
            r = A.prompt("t306_spreadsheet", "R")
            u = A.prompt("t306_spreadsheet", "U7")
        finally:
            T.TASKS = old
        for text, route in ((r, "cat in.txt | ./main"), (u, "--stdin-file in.txt")):
            self.assertIn("\\075", text)
            self.assertIn(route, text)


class RuleCost(unittest.TestCase):
    def test_input_writes_counts_non_program_writes_only(self):
        ev = lambda p: json.dumps({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": p, "name": "Write", "input": {"file_path": p}}]}})
        stream = "\n".join(ev(p) for p in ("/w/main.wj", "/w/ex1.txt", "/w/in.txt", "/w/main.rs", "/w/x_test.wj"))
        self.assertEqual(DN.input_writes(stream), 2)

    def test_report_table_has_the_input_writes_column(self):
        from agentdx import pilot7_report as R7
        den = {("haiku", "U7"): {"n": 2, "denied": 0, "first_denied": 0, "never": 0, "never_denied": 0, "input_writes": 3, "passed": 2, "shapes": {}, "commands": {}}}
        md = "\n".join(R7.denial_section(den))
        self.assertIn("input-file writes per trial", md)
        self.assertIn("| 1.50 |", md)
        self.assertTrue(any(l.rstrip().endswith("| 1.50 |") for l in md.splitlines()))


if __name__ == "__main__":
    unittest.main()
