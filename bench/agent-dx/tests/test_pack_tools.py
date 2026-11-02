"""Pack-size experiment: tiny packs, tool access per arm, stream parsing, attribution fields."""
import json
import os
import tempfile
import unittest

from agentdx import arms, pilot, stream
from tests.test_pilot import FakeAgent, fake_wj, grader

T = "t003_prime_counts"
READ = lambda n: open(os.path.join(arms.PACKS_DIR, n)).read()


class TinyPacks(unittest.TestCase):
    def test_sizes_and_shared_base(self):
        base = READ("tiny_base.md")
        for n in ("tiny_E.md", "tiny_F.md", "tiny_T.md"):
            t = READ(n)
            self.assertLessEqual(len(t.encode()), 2000, n)
            self.assertTrue(t.startswith(base), n)
        self.assertGreater(len(READ("tiny_E.md")), len(base))
        self.assertEqual(READ("tiny_T.md"), base)

    def test_only_tool_instruction_differs_between_e_and_f(self):
        e, f = READ("tiny_E.md").splitlines(), READ("tiny_F.md").splitlines()
        diff = [(x, y) for x, y in zip(e, f) if x != y]
        self.assertEqual(len(diff), 1)
        self.assertIn("MCP", diff[0][0]); self.assertIn("wj docs", diff[0][1])

    def test_no_task_hints_in_tiny_packs(self):
        from tests.test_pack_guardrails import leaks
        for n in ("tiny_E.md", "tiny_F.md", "tiny_T.md"):
            self.assertEqual(leaks(READ(n)), [])


class ToolAccess(unittest.TestCase):
    def test_same_loop_paragraph_and_prompt_for_b_e_f_t(self):
        ps = {a: arms.prompt(T, a) for a in "BEFT"}
        self.assertEqual(len(set(ps.values())), 1)

    def test_t_has_no_docs_f_has_shell_docs_e_has_only_mcp(self):
        t, f, e, b = (arms.tool_policy(a)["allowed"] for a in "TFEB")
        self.assertNotIn("Bash(wj docs:*)", t + e)
        self.assertNotIn("Bash(wj explain:*)", t + e)
        self.assertIn("Bash(wj docs:*)", f)
        self.assertIn("Bash(wj:*)", b)
        self.assertFalse([x for x in t + f if x.startswith("mcp__")])
        self.assertEqual(sorted(x for x in e if x.startswith("mcp__")), sorted(f"mcp__windjammer__{n}" for n in arms.MCP_TOOLS))

    def test_e_argv_attaches_mcp_and_hides_the_other_tools(self):
        argv = arms.claude_argv("E", T, "sonnet", 25, 2.0, mcp_config_path="/x/mcp.json", stream=True)
        self.assertEqual(argv[argv.index("--mcp-config") + 1], "/x/mcp.json")
        hidden = argv[argv.index("--disallowedTools") + 1].split(",")
        self.assertIn("mcp__windjammer__run_code", hidden)
        for n in arms.MCP_TOOLS:
            self.assertNotIn(f"mcp__windjammer__{n}", hidden)
        self.assertEqual(argv[argv.index("--append-system-prompt-file") + 1], os.path.join(arms.PACKS_DIR, "tiny_E.md"))
        self.assertIn("stream-json", argv); self.assertIn("--verbose", argv)
        self.assertNotIn("--mcp-config", arms.claude_argv("F", T, "sonnet", 25, 2.0, stream=True))

    def test_every_catalog_tool_is_either_exposed_or_hidden(self):
        names = set(arms.all_mcp_tools())
        self.assertTrue(set(arms.MCP_TOOLS) <= names)


STREAM = "\n".join(json.dumps(x) for x in [
    {"type": "system", "subtype": "init", "tools": ["Read", "mcp__windjammer__docs_search"]},
    {"type": "assistant", "message": {"id": "m1", "usage": {"input_tokens": 5, "cache_creation_input_tokens": 1000, "cache_read_input_tokens": 0, "output_tokens": 50},
                                      "content": [{"type": "text", "text": "x"}]}},
    {"type": "assistant", "message": {"id": "m1", "usage": {"input_tokens": 5, "cache_creation_input_tokens": 1000, "cache_read_input_tokens": 0, "output_tokens": 60},
                                      "content": [{"type": "tool_use", "name": "mcp__windjammer__docs_search", "input": {}}]}},
    {"type": "user", "message": {"content": [{"type": "tool_result", "content": [{"type": "text", "text": "a" * 700}]}]}},
    {"type": "assistant", "message": {"id": "m2", "usage": {"input_tokens": 3, "cache_creation_input_tokens": 200, "cache_read_input_tokens": 1000, "output_tokens": 20},
                                      "content": [{"type": "tool_use", "name": "Bash", "input": {}}]}},
    {"type": "user", "message": {"content": [{"type": "tool_result", "content": "bbbb"}]}},
    {"type": "result", "subtype": "success", "num_turns": 2, "total_cost_usd": 0.01, "usage": {"input_tokens": 8, "output_tokens": 80}},
])


class StreamParse(unittest.TestCase):
    def test_turns_calls_and_results(self):
        res, log, tools = stream.parse(STREAM + "\nnot json\n")
        self.assertEqual(res["num_turns"], 2)
        self.assertEqual([t["input_total"] for t in log], [1005, 1203])
        self.assertEqual(log[0]["output"], 60)                       # last usage of a message id wins, no double count
        self.assertEqual(log[0]["calls"], ["mcp__windjammer__docs_search"])
        self.assertEqual(log[0]["result_chars"], 700); self.assertEqual(log[1]["result_chars"], 4)
        s = stream.summarise(log, tools)
        self.assertEqual(s["first_turn_input"], 1005)
        self.assertEqual(s["mcp_calls"], {"mcp__windjammer__docs_search": 1})
        self.assertEqual(s["tool_result_tokens_est"], 201)

    def test_empty_and_truncated_streams(self):
        self.assertEqual(stream.parse("")[0], None)
        self.assertEqual(stream.summarise([], [])["first_turn_input"], None)


class TrialAttribution(unittest.TestCase):
    def test_record_carries_pack_and_docs_fields(self):
        with tempfile.TemporaryDirectory() as d:
            real = fake_wj(d)
            tr = pilot.run_trial(T, "F", FakeAgent([["docs", "hashmap"], ["explain", "WJ0001"], ["build", "main.wj"]]), 0, d, grader(), real_wj=real)
            self.assertEqual(tr["docs_calls"], 2)
            self.assertEqual(tr["pack_file"], "tiny_F.md")
            self.assertGreater(tr["pack_bytes"], 1000)
            self.assertEqual(pilot.run_trial(T, "B", FakeAgent([]), 1, d, grader(), real_wj=real)["pack_file"], "WINDJAMMER_FOR_AGENTS.md")


if __name__ == "__main__":
    unittest.main()


class ReportAttribution(unittest.TestCase):
    def test_attribution_and_vs_control_tables(self):
        from agentdx import pilot_report
        from tests.test_arms_aps import trial
        ts = []
        for a, ft, pk in (("B", 9000, 15000), ("E", 6500, 1544), ("T", 5000, 1297)):
            for i in range(4):
                t = trial(a, f"t{i}", True)
                t.update({"first_turn_input": ft, "pack_bytes": pk, "tool_result_tokens_est": 100, "docs_calls": 1, "mcp_calls": {"x": 2} if a == "E" else {}})
                ts.append(t)
        md, js = pilot_report.build_report(ts, 100)
        self.assertIn("Context attribution", md)
        self.assertIn("versus control B", md)
        e_schema = (6500 - 5000) - (1544 - 1297) / 3.5
        self.assertAlmostEqual(js["mcp_schema_tokens"], e_schema)
