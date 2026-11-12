"""Pilot 7 spend integrity, seed, wj identity and preflight. No model is called: a stub `claude` script and fake agents."""
import contextlib
import io
import json
import os
import stat
import tempfile
import unittest
from unittest import mock

from agentdx import cli, pilot as PL, pilot7 as P7, preflight as PF, spend as SP, tasks as T, toolchains as TC
from tests.test_pilot import FakeAgent, fake_wj, grader


def cfg(models=("haiku",), arms=("U7",), tasks=("t101_json_orders",), **kw):
    c = json.loads(json.dumps(P7.load_config()))
    c["tasks"], c["tasks_dir"], c["arms"] = list(tasks), "tasks_set2", list(arms)
    c["models"] = {m: dict(c["models"][m], trials_per_cell=1) for m in models}
    c["trials_per_cell"] = 1
    c.update(kw)
    return c


def stub_claude(d, body):
    p = os.path.join(d, "claude")
    open(p, "w").write("#!/bin/sh\n" + body)
    os.chmod(p, 0o755)
    return p


PARTIAL = ('echo \'{"type":"assistant","message":{"id":"m1","usage":{"input_tokens":1000,"cache_creation_input_tokens":0,'
           '"cache_read_input_tokens":0,"output_tokens":2000},"content":[]}}\'\n')


class TimeoutAgent:
    """Agent that times out with a given charge (what ClaudeAgent returns when claude exceeds its timeout)."""
    model = "haiku"

    def __init__(self, usd=0.07):
        self.usd, self.runs = usd, 0

    def run(self, ws, shim_dir, arm, task):
        self.runs += 1
        open(os.path.join(ws, "main.wj"), "w").write("fn main() {}\n")
        return {"json": None, "exit": -9, "log": "agent timeout", "timed_out": True, "timeout_cost_usd": self.usd, "cost_basis": "partial_stream"}


LINE = "wj 0.50.0 (commit 79d7484383d5 2026-11-12, clean, release, built 2026-11-12T13:53:30Z)"


def make_wj(d, line=LINE):
    p = os.path.join(d, "wjx")
    open(p, "w").write(f"#!/bin/sh\necho '{line}'\n")
    os.chmod(p, 0o755)
    return p


class Env(unittest.TestCase):
    def setUp(self):
        saved = (T.TASKS, os.environ.get("AGENTDX_TASKS_DIR"))   # run_matrix points task loading at the pilot's set
        self.addCleanup(self._restore, saved)
        P7.use_tasks(cfg())
        self.tmp = tempfile.TemporaryDirectory()
        self.d = self.tmp.name
        self.real = fake_wj(self.d)
        self.out = os.path.join(self.d, "out")
        self.quiet = lambda *_: None
        self.addCleanup(self.tmp.cleanup)

    @staticmethod
    def _restore(saved):
        T.TASKS = saved[0]
        if saved[1] is None:
            os.environ.pop("AGENTDX_TASKS_DIR", None)
        else:
            os.environ["AGENTDX_TASKS_DIR"] = saved[1]

    def run_m(self, c, agent_for, **kw):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            r = P7.run_matrix(c, self.out, agent_for, say=self.quiet, grader=grader(), real_wj=self.real, **kw)
        return r, err.getvalue()

    def trial(self, m="haiku"):
        return PL.load_trials(os.path.join(self.out, m))[0]


class TimeoutTest(Env):
    def test_stub_claude_timeout_is_charged_from_the_partial_stream(self):
        bin_ = stub_claude(self.d, PARTIAL + "exec sleep 30\n")
        c = cfg()
        r, _ = self.run_m(c, lambda m: PL.ClaudeAgent(m, 25, 0.1, claude_bin=bin_, timeout=2, stream=True, prices=c["prices"][m]))
        t = self.trial()
        self.assertEqual((t["status"], t["excluded_from_grading"], t["agent_exit"]), ("timeout", True, -9))
        self.assertEqual(t["usage"]["cost_basis"], "partial_stream")
        self.assertAlmostEqual(t["usage"]["cost_usd"], 1000 * 0.1 / 1e6 + 2000 * 0.5 / 1e6, places=8)
        self.assertAlmostEqual(r["spent_usd"], t["usage"]["cost_usd"], places=8)

    def test_timeout_with_no_stream_is_charged_the_per_trial_cap(self):
        bin_ = stub_claude(self.d, "exec sleep 30\n")
        c = cfg()
        r, _ = self.run_m(c, lambda m: PL.ClaudeAgent(m, 25, 0.1, claude_bin=bin_, timeout=1, stream=True, prices=c["prices"][m]))
        self.assertEqual(self.trial()["usage"]["cost_basis"], "per_trial_cap")
        self.assertAlmostEqual(r["spent_usd"], 0.1)

    def test_resume_does_not_rerun_a_timeout_unless_configured_and_keeps_its_spend(self):
        c = cfg()
        a = TimeoutAgent()
        self.run_m(c, lambda m: a)
        self.assertEqual(a.runs, 1)
        r, _ = self.run_m(c, lambda m: a)
        self.assertEqual((a.runs, round(r["spent_usd"], 4)), (1, 0.07))
        r, _ = self.run_m(dict(c, rerun_timeouts=True), lambda m: a)
        self.assertEqual(a.runs, 2)
        self.assertAlmostEqual(r["spent_usd"], 0.14)   # both attempts are paid and counted
        self.assertEqual(r["done"]["haiku"], 1)


class LedgerTest(Env):
    def test_killed_trial_counts_at_its_cap_and_a_normal_trial_is_not_double_counted(self):
        c = cfg()
        self.run_m(c, lambda m: FakeAgent([], cost=0.04))
        led = SP.Ledger(os.path.join(self.out, "spend.jsonl"))
        self.assertEqual([(a["status"], a["usd"]) for a in led.attempts()], [("completed", 0.04)])
        led.start("haiku/other.U7.0", "haiku", 0.1, os.path.join(self.d, "missing.jsonl"))   # process killed here
        g = P7.MatrixGuard(c, P7.matrix(c))
        P7.restore_guard(g, c, self.out, led, self.quiet)
        self.assertAlmostEqual(g.spent, 0.04 + 0.1)
        self.assertEqual(g.done["haiku"], 1)

    def test_unfinished_attempt_is_reconciled_from_its_stream_log(self):
        c = cfg()
        led = SP.Ledger(os.path.join(self.out, "spend.jsonl"))
        s = os.path.join(self.d, "claude.stream.jsonl")
        open(s, "w").write('{"type":"result","total_cost_usd":0.0123,"usage":{}}\n')
        led.start("haiku/x.U7.0", "haiku", 0.1, s)
        g = P7.MatrixGuard(c, P7.matrix(c))
        P7.restore_guard(g, c, self.out, led, self.quiet)
        self.assertAlmostEqual(g.spent, 0.0123)
        self.assertEqual(led.attempts()[0]["status"], "interrupted")

    def test_ledger_line_is_on_disk_before_the_model_runs(self):
        seen = []

        class Peek(FakeAgent):
            def run(s, ws, shim_dir, arm, task):
                seen.append(open(os.path.join(self.out, "spend.jsonl")).read())
                return super().run(ws, shim_dir, arm, task)
        self.run_m(cfg(), lambda m: Peek([]))
        self.assertIn('"ev": "start"', seen[0])
        self.assertNotIn('"ev": "end"', seen[0])

    def test_interrupted_trial_is_settled_and_reraised(self):
        class Boom(FakeAgent):
            def run(s, *a):
                raise KeyboardInterrupt
        with self.assertRaises(KeyboardInterrupt):
            self.run_m(cfg(), lambda m: Boom([]))
        a = SP.Ledger(os.path.join(self.out, "spend.jsonl")).attempts()[0]
        self.assertEqual((a["status"], a["usd"]), ("error", 0.1))   # no stream: the cap


class StopTest(Env):
    def test_stopped_model_is_loud_in_stderr_final_json_marker_and_exit_code(self):
        c = cfg(hard_cap_usd=0.05)
        r, err = self.run_m(c, lambda m: FakeAgent([]))
        self.assertIn("STOPPED model=haiku", err)
        for word in ("spent=", "cap=", "projection="):
            self.assertIn(word, err)
        self.assertEqual(r["stopped"][0]["model"], "haiku")
        self.assertEqual(json.load(open(os.path.join(self.out, "final.json")))["stopped"][0]["cap_usd"], 0.05)
        self.assertTrue(os.path.exists(os.path.join(self.out, "STOPPED")))
        with mock.patch.object(P7, "dry_run", return_value=[]), mock.patch.object(P7, "run_matrix", return_value=r), \
                mock.patch.object(TC, "wj_identity", return_value={"path": "wj", "version": "wj x", "commit": "", "clean": True, "release": True}), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(cli.main(["pilot7", "--yes"]), 4)

    def test_no_stop_no_marker_exit_zero(self):
        r, err = self.run_m(cfg(), lambda m: FakeAgent([], cost=0.01))
        self.assertEqual(r["stopped"], [])
        self.assertNotIn("STOPPED", err)
        self.assertFalse(os.path.exists(os.path.join(self.out, "STOPPED")))


class SeedTest(Env):
    def test_seed_changes_the_order_defaults_to_zero_and_is_recorded(self):
        c = cfg(tasks=[f"t{i}" for i in range(10)])
        self.assertEqual(P7.seed_of({}), 0)
        self.assertEqual(P7.matrix(c), P7.matrix(dict(c, seed=0)))
        self.assertNotEqual(P7.matrix(c), P7.matrix(dict(c, seed=7)))
        r, _ = self.run_m(dict(cfg(), seed=7), lambda m: FakeAgent([], cost=0.01))
        self.assertEqual((self.trial()["seed"], r["seed"]), (7, 7))


class VersionTest(Env):
    LINE = LINE

    def wj(self):
        return make_wj(self.d)

    def test_identity_is_parsed_from_the_binary(self):
        i = TC.wj_identity(self.wj())
        self.assertEqual((i["commit"], i["clean"], i["release"]), ("79d7484383d5", True, True))

    def test_env_that_disagrees_is_refused_and_agreeing_env_passes(self):
        i = TC.wj_identity(self.wj())
        self.assertTrue(TC.env_disagreements(i, {"AGENTDX_WJ_VERSION": "wj 0.49"}))
        self.assertEqual(TC.env_disagreements(i, {"AGENTDX_WJ_VERSION": self.LINE, "AGENTDX_WJ": i["path"]}), [])
        self.assertTrue(TC.env_disagreements(i, {"AGENTDX_WJ": "/somewhere/else/wj"}))

    def test_trial_record_holds_the_derived_version_not_the_env_text(self):
        p = self.wj()
        with mock.patch.dict(os.environ, {"AGENTDX_WJ_VERSION": "lies"}):
            tr = PL.run_trial("t101_json_orders", "A", FakeAgent([]), 0, self.d, grader(), real_wj=p)
        self.assertEqual((tr["wj_version"], tr["wj_commit"]), (self.LINE, "79d7484383d5"))


class PreflightTest(Env):
    def test_wj_checks(self):
        i = TC.wj_identity(make_wj(self.d))
        self.assertEqual(PF.check_wj(i, "79d7484", {}), [])
        self.assertTrue(PF.check_wj(i, "b5444baa7", {}))
        dirty = dict(i, clean=False)
        self.assertTrue(PF.check_wj(dirty, "79d7", {}))
        self.assertTrue(PF.check_wj(dict(i, version=""), "79d7", {}))

    def test_load_and_memory(self):
        self.assertEqual(PF.check_load(3.0), [])
        self.assertTrue(PF.check_load(23.0))
        self.assertTrue(PF.check_memory(1.0, 4.0))
        self.assertEqual(PF.check_memory(8.0, 4.0), [])
        self.assertTrue(PF.check_memory(None, 4.0))
        self.assertEqual(PF.check_memory(None, 0), [])

    def test_events_need_an_accepted_prefix_with_a_reason(self):
        c = cfg(tasks=["t104_activity_log"])
        ids = ["p6par-t104-V2-1-c00", "p3-t003-B-2-c00"]   # the second is on a task outside the matrix
        self.assertEqual(len(PF.check_events(c, ids, [])), 1)
        acc = os.path.join(self.d, "acc.txt")
        open(acc, "w").write("# c\n\np6par-t104-V2 known widths bug\nbadline\n")
        ok, bad = PF.read_accepted(acc)
        self.assertEqual(PF.check_events(c, ids, ok), [])
        self.assertEqual(len(bad), 1)

    def test_open_bugs_file_is_parsed(self):
        p = os.path.join(self.d, "ob.md")
        open(p, "w").write("| Event | x |\n|---|---|\n| `p6par-t104-V2-1-c00` | widths |\n")
        self.assertEqual(PF.open_event_ids(p), ["p6par-t104-V2-1-c00"])

    def test_run_exits_nonzero_on_any_red_and_names_it(self):
        args = type("A", (), dict(config=P7.CONFIG, expect_commit="deadbee", accepted=os.path.join(self.d, "none"),
                                  open_bugs=os.path.join(self.d, "none"), min_free_mem_gb=4.0, prior=[]))
        out = []
        with mock.patch.object(TC, "wj_identity", return_value=TC.wj_identity(make_wj(self.d))):
            rc = PF.run(args, load1=30.0, mem_gb=1.0, say=out.append)
        self.assertEqual(rc, 1)
        text = "\n".join(out)
        for name in ("RED   wj", "RED   load", "RED   memory", "RED   events"):
            self.assertIn(name, text)
        self.assertIn("PREFLIGHT RED:", text)


if __name__ == "__main__":
    unittest.main()
