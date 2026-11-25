"""Pilot 12 false 'never compiled': a build or run that worked behind `| head`, `2>/dev/null` or `; echo "[exit $?]"` was not
counted as a success. The classifier now reads the tool RESULT (not the command's shape). Fixtures are neutral stream-json
transcripts of those shapes; the prompt rules (`>` is never written, `--stdin-text` only behind the harness flag) are checked too."""
import json
import os
import subprocess
import sys
import tempfile
import unittest

from agentdx import arms as A, denials as DN, shim as S

FIX = os.path.join(os.path.dirname(__file__), "fixtures", "denials")


def fixture(name):
    return open(os.path.join(FIX, name + ".jsonl")).read()


class ResultClassifier(unittest.TestCase):
    def test_successful_calls_behind_head_devnull_and_exit_echo_count(self):
        for name in ("ok_build_piped_to_head", "ok_run_with_exit_echo", "ok_run_quiet_stderr"):
            self.assertTrue(DN.stream_compiled_ok(fixture(name)), name)

    def test_failing_calls_in_the_same_shapes_do_not_count(self):
        for name in ("fail_build_piped_to_head", "fail_run_with_exit_echo"):
            self.assertFalse(DN.stream_compiled_ok(fixture(name)), name)

    def test_a_denied_call_and_a_non_verify_command_do_not_count(self):
        self.assertFalse(DN.stream_compiled_ok(fixture("dontask_first_compound_wj")))
        self.assertFalse(DN.stream_compiled_ok(fixture("clean_first_call_ok").replace("wj build main.wj 2>&1 | head -40", "ls")))

    def test_result_rules(self):
        self.assertTrue(DN.result_shows_success("3 apples\n[exit 0]"))
        self.assertFalse(DN.result_shows_success("3 apples\n[exit 2]"))
        self.assertFalse(DN.result_shows_success("ok\nexit status: 1"))
        self.assertTrue(DN.result_shows_success("ok\nexit status: 0"))
        self.assertFalse(DN.result_shows_success("", piped=True), "empty output behind a pipe or redirect hides a failure")
        self.assertTrue(DN.result_shows_success("", piped=False))
        self.assertFalse(DN.result_shows_success("whatever", is_error=True))
        self.assertTrue(DN.result_shows_success("Exit code 1\nok a 10\nerror: account exists a", is_error=True, runs=True), "Claude Code flags exit 1 as is_error")
        self.assertFalse(DN.result_shows_success("<tool_use_error>Blocked: sleep</tool_use_error>", is_error=True, runs=True))

    def test_mutation_a_classifier_that_ignores_the_result_is_caught(self):
        orig = DN.result_shows_success
        try:
            DN.result_shows_success = lambda *a, **k: False
            self.assertFalse(DN.stream_compiled_ok(fixture("ok_build_piped_to_head")))   # the positive tests above would fail
            DN.result_shows_success = lambda *a, **k: True
            self.assertTrue(DN.stream_compiled_ok(fixture("fail_build_piped_to_head")))  # the negative tests above would fail
        finally:
            DN.result_shows_success = orig


class ShimLogsBehindHead(unittest.TestCase):
    def test_a_build_piped_into_head_is_still_logged(self):
        with tempfile.TemporaryDirectory() as d:
            real = os.path.join(d, "real")
            open(real, "w").write("#!/bin/sh\nseq 1 400000\nexit 0\n")
            os.chmod(real, 0o755)
            log = os.path.join(d, "log.jsonl")
            S.install(os.path.join(d, "bin"), real, log, os.path.join(d, "main.wj"))
            subprocess.run(f"{os.path.join(d, 'bin', 'wj')} build main.wj | head -n 3", shell=True, capture_output=True)
            rows = S.read_log(log)
            self.assertEqual([(r["verb"], r["exit"]) for r in rows], [("build", 0)])


class PromptRules(unittest.TestCase):
    def tearDown(self):
        A.STDIN_TEXT_HINT = False

    def test_never_write_redirect_rule_is_in_every_wj_prompt_and_hint_is_off_by_default(self):
        p = A.prompt(sorted(os.listdir(os.path.join(A.ROOT, "tasks")))[0], "U7")
        self.assertIn("NEVER write `>`", p)
        self.assertNotIn("--stdin-text", p)
        A.STDIN_TEXT_HINT = True
        self.assertIn("--stdin-text 'line one", A.prompt(sorted(os.listdir(os.path.join(A.ROOT, "tasks")))[0], "U7"))

    def test_hint_is_for_wj_arms_only(self):
        A.STDIN_TEXT_HINT = True
        self.assertNotIn("--stdin-text", A.prompt(sorted(os.listdir(os.path.join(A.ROOT, "tasks")))[0], "R"))


class ExitCodeIsNotTheCompile(unittest.TestCase):
    """Pilot 13: t302/t305 U7 trials ran correctly and exited 1 (the spec's error-exit tasks) and were flagged never-compiled."""

    def test_a_correct_run_that_exits_1_with_program_error_lines_counts(self):
        for name in ("ok_run_exit1_program_error_lines", "ok_run_exit1_stdin_file"):
            self.assertTrue(DN.stream_compiled_ok(fixture(name)), name)

    def test_a_real_compile_failure_and_a_silent_failure_do_not_count(self):
        for name in ("fail_run_coded_compile_error_exit1", "fail_run_agent_format_error_exit1", "fail_run_silent_exit1_no_output",
                     "dontask_first_compound_wj"):
            self.assertFalse(DN.stream_compiled_ok(fixture(name)), name)

    def test_a_build_only_command_still_fails_on_an_uncoded_error_line(self):
        self.assertFalse(DN.result_shows_success("error: expected `}`", runs=False))
        self.assertTrue(DN.result_shows_success("error: expected `}`", runs=True), "for a run this is the program's own output")
        self.assertFalse(DN.result_shows_success("error[E0308]: mismatched types", runs=True))

    def test_runs_program_recognises_the_run_shapes(self):
        for cmd in ("wj run --compiled main.wj", "cat in.txt | ./main", "wj run --compiled --stdin-file in.txt main.wj", "./build/target/debug/main"):
            self.assertTrue(DN.runs_program(cmd), cmd)
        for cmd in ("wj build main.wj 2>&1 | head -40", "wj check main.wj", "rustc main.rs"):
            self.assertFalse(DN.runs_program(cmd), cmd)

    def test_mutation_judging_by_the_exit_code_again_is_caught(self):
        orig = DN.runs_program
        try:
            DN.runs_program = lambda cmd: False
            self.assertFalse(DN.stream_compiled_ok(fixture("ok_run_exit1_program_error_lines")),
                             "with the old rule the exit-1 run is not counted: the mutation must flip the first test")
        finally:
            DN.runs_program = orig
        self.assertTrue(DN.stream_compiled_ok(fixture("ok_run_exit1_program_error_lines")))


if __name__ == "__main__":
    unittest.main()
