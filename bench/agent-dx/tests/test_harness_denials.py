"""Permission denials of compound shell commands: detected (both kinds, both sources), recorded per trial, flagged per arm; the allow set is
symmetric by construction. Fixtures in tests/fixtures/denials are stream-json transcripts built from the recorded pilot 8-10 denials (real command
lines and the verbatim don't-ask tool result); no model is called."""
import json
import os
import shlex
import unittest

from agentdx import arms as A, denials as DN, pilot as PL, pilot7_report as P7R
from tests.test_pilot7_safety import Env, cfg, stub_claude

FIX = os.path.join(os.path.dirname(__file__), "fixtures", "denials")


def fixture(name):
    return open(os.path.join(FIX, name)).read()


class DetectorTest(unittest.TestCase):
    def test_dont_ask_first_call_compound_is_counted_with_its_shape(self):
        d = DN.summarise(fixture("dontask_first_compound_wj.jsonl"))
        self.assertEqual((d["count"], d["kinds"], d["first_bash_denied"]), (1, {"dont-ask": 1}, True))
        (shape,) = d["shapes"]
        self.assertTrue(shape.startswith("wj | head ; echo ; printf | ./main"), shape)
        self.assertIn("[var]", shape)

    def test_system_permission_denied_event_is_counted_and_a_string_message_does_not_crash(self):
        # Claude Code emits {"type":"system","subtype":"permission_denied",...,"message":"<string>"}: the parser used to
        # call .get on that string and crash the whole pilot (pilot 11).
        d = DN.summarise(fixture("system_permission_denied_event.jsonl"))
        self.assertEqual((d["count"], d["kinds"], d["first_bash_denied"]), (1, {"dont-ask": 1}, True))
        self.assertTrue(any("[var]" in s for s in d["shapes"]), d["shapes"])

    def test_space_equals_in_a_printf_pipeline_is_marked(self):
        d = DN.summarise(fixture("dontask_printf_equals_wj.jsonl"))
        self.assertEqual(d["count"], 1)
        self.assertIn("[space-eq]", list(d["shapes"])[0])
        self.assertTrue(d["first_bash_denied"])

    def test_rust_arm_denial_is_counted_the_same_way(self):
        d = DN.summarise(fixture("dontask_compound_rust.jsonl"))
        self.assertEqual((d["count"], d["first_bash_denied"]), (1, True))
        self.assertTrue(list(d["shapes"])[0].startswith("rustc | head && printf | ./main"))

    def test_capability_denial_is_its_own_kind_and_not_a_permission_denial(self):
        d = DN.summarise(fixture("capability_and_clean.jsonl"))
        self.assertEqual((d["count"], d["capability"], d["first_bash_denied"]), (0, 1, False))

    def test_clean_transcript_has_no_denials(self):
        self.assertEqual(DN.summarise(fixture("clean_first_call_ok.jsonl"))["count"], 0)

    def test_result_json_permission_denials_alone_are_counted_once(self):
        j = {"permission_denials": [{"tool_name": "Bash", "tool_input": {"command": "a && b"}}]}
        d = DN.summarise("", j)
        self.assertEqual((d["count"], d["commands"]), (1, ["a && b"]))
        both = DN.summarise(fixture("dontask_first_compound_wj.jsonl"), json.loads(fixture("dontask_first_compound_wj.jsonl").splitlines()[-1]))
        self.assertEqual(both["count"], 1)   # stream tool result and result.permission_denials: same tool_use_id

    def test_matcher_needs_both_phrases(self):
        self.assertEqual(DN.classify_text("Permission to use Bash has been denied because Claude Code is running in don’t ask mode."), "dont-ask")
        self.assertEqual(DN.classify_text("Permission to use Bash has been denied by the user."), "permission")
        self.assertIsNone(DN.classify_text("the file has not been denied; don't ask"))

    def test_split_is_quote_aware(self):
        self.assertEqual(DN.sub_commands("printf 'a && b | c' | ./main && echo \"x; y\"; ls"), ["printf", "./main", "echo", "ls"])


class ReportTest(Env):
    def write(self, arm, k, fixture_name, never=True):
        d = DN.summarise(fixture(fixture_name))
        t = {"task": "t101_json_orders", "arm": arm, "trial": k, "pass_final": not never, "never_compiled": never, "denials": d, "usage": {}, "flags": []}
        os.makedirs(os.path.join(self.out, "haiku", "trials"), exist_ok=True)
        json.dump(t, open(os.path.join(self.out, "haiku", "trials", f"t101_json_orders.{arm}.{k}.json"), "w"))

    def test_arm_with_first_call_denials_is_harness_limited_and_withheld(self):
        c = cfg(arms=("U7", "R"))
        for k in range(10):
            self.write("R", k, "clean_first_call_ok.jsonl", never=False)
        self.write("U7", 0, "dontask_first_compound_wj.jsonl")
        for k in range(1, 10):
            self.write("U7", k, "clean_first_call_ok.jsonl", never=False)
        refusals = []
        md = P7R.build(self.out, c, refusals=refusals)
        self.assertIn("HARNESS-LIMITED", md)
        self.assertEqual(len(refusals), 1)
        self.assertTrue(refusals[0].startswith("haiku/U7"), refusals)
        self.assertIn("never-compiled trials", md)
        row = [ln for ln in md.splitlines() if ln.startswith("| haiku | U7 |")][0].split("|")
        self.assertEqual(row[5].strip(), "1")      # never-compiled trials
        self.assertEqual(row[6].strip(), "1")      # of them with a denial
        self.assertEqual(row[9].strip(), "HARNESS-LIMITED")
        self.assertIn("haiku/R", "".join(refusals) + "haiku/R")    # R row printed as ok
        self.assertTrue([ln for ln in md.splitlines() if ln.startswith("| haiku | R |")][0].split("|")[9].strip() == "ok")

    def test_no_denial_arm_is_not_limited(self):
        self.assertFalse(P7R.harness_limited({"n": 20, "first_denied": 1}))
        self.assertTrue(P7R.harness_limited({"n": 20, "first_denied": 2}))

    def test_trial_record_gets_denials_flag_and_never_compiled_from_a_replayed_stream(self):
        c = cfg()
        bin_ = stub_claude(self.d, f"cat {shlex.quote(os.path.join(FIX, 'dontask_first_compound_wj.jsonl'))}\n")
        self.run_m(c, lambda m: PL.ClaudeAgent(m, 25, 0.1, claude_bin=bin_, timeout=20, stream=True, prices=c["prices"][m]))
        t = self.trial()
        self.assertEqual(t["flags"], ["harness-denial"])
        self.assertEqual((t["denials"]["count"], t["denials"]["first_bash_denied"], t["never_compiled"]), (1, True, True))


class AllowSetTest(unittest.TestCase):
    HELPERS = {f"Bash({c}:*)" for c in A.SHELL_HELPERS}

    def bash(self, arm):
        return [x for x in A.tool_policy(arm)["allowed"] if x.startswith("Bash(")]

    def toolchain(self, arm):
        return {x for x in self.bash(arm) if x not in self.HELPERS}

    def test_every_arm_gets_exactly_the_same_helpers_plus_only_its_own_toolchain(self):
        for a in A.ARMS:
            self.assertTrue(self.HELPERS <= set(self.bash(a)), a)
            self.assertEqual(len(self.bash(a)), len(set(self.bash(a))), f"duplicates for {a}")
        self.assertTrue(all(x.startswith("Bash(wj") for x in self.toolchain("U7")))
        self.assertTrue(all(x.startswith("Bash(wj") for x in self.toolchain("V3")))
        self.assertEqual(self.toolchain("R"), {"Bash(rustc:*)"})
        self.assertEqual(self.toolchain("G"), {"Bash(go:*)"})
        self.assertEqual(self.toolchain("P"), {"Bash(python3:*)"})

    def test_wj_arms_share_one_toolchain_set_and_it_has_no_extra_verbs_over_rustc(self):
        self.assertEqual(self.toolchain("U7"), self.toolchain("V3"))
        verbs = {x[len("Bash(wj "):-3] for x in self.toolchain("U7")}
        self.assertEqual(verbs, set(A.WJ_CORE) | {"docs", "explain"})

    def test_every_chained_part_of_a_verification_line_is_allowed_for_both_arms(self):
        lines = ["wj build main.wj 2>&1 | head -40 && printf 'a\\n' | ./main", "rustc -o main main.rs 2>&1 | head -40 && printf 'a\\n' | ./main",
                 "wj build main.wj; echo ---; cat input.txt | ./build/main | diff - expected.txt | wc -l", "cd . && ls | sort | uniq | tr a b | cut -d, -f1 | grep x | tail -3"]
        for line in lines:
            arm = "R" if DN.sub_commands(line)[0] == "rustc" else "U7"
            allowed = {x[5:-3] for x in self.bash(arm)}
            for w in DN.sub_commands(line):
                self.assertTrue(w in allowed or (w == "wj" and "wj build" in allowed), f"{w} in {line} not allowed for {arm}")

    def test_nothing_destructive_or_networked_is_allowed(self):
        for arm in ("U7", "V3", "R", "P", "G"):
            joined = " ".join(A.tool_policy(arm)["allowed"])
            for bad in ("curl", "wget", "ssh", "git", "rm:", "mv:", "cp:", "chmod", "kill", "pip", "npm", "cargo", "timeout", "xargs", "find", "awk", "env:", "sh:", "bash:"):
                self.assertNotIn(f"Bash({bad.rstrip(':')}", joined, f"{bad} allowed for {arm}")


if __name__ == "__main__":
    unittest.main()
