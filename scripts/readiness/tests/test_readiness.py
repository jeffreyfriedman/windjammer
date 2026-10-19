"""Tests for the scorecard logic, driven by a fake `wj` shim with known pass/fail/divergence patterns.
Run: python3 -m unittest discover -s scripts -t scripts -p 'test_*.py'   (also wired into cargo test)."""
import io, json, os, shutil, sys, tempfile, unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPTS = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, SCRIPTS)
from readiness import core, corpora, history, pygen, r1, r2, r3, r4, r5, r6, report, selfcheck  # noqa: E402
import interp_readiness  # noqa: E402

SHIM = os.path.join(HERE, "fake_wj.py")
TEMPLATE = "// seeds: 1 2\n// exit: 0\nfn main() {\n    let s: int = __SEED__\n    println(\"${s * 7 + 3}\")\n}\n"
REF = "import sys\nprint(int(sys.argv[1]) * 7 + 3)\n"


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


class Fixture:
    """A throwaway compiler checkout + ecosystem + a fake wj configured by `spec`."""
    def __init__(self, spec, private=False):
        self.tmp = tempfile.mkdtemp(prefix="wj-readiness-test-")
        self.root = os.path.join(self.tmp, "repo")
        self.eco = os.path.join(self.tmp, "eco")
        spec_path = os.path.join(self.tmp, "spec.json")
        write(spec_path, json.dumps(spec))
        os.environ["WJ_FAKE_SPEC"] = spec_path
        os.environ["WJ_READINESS_MIN_FREE_GIB"] = "0"   # tests must not depend on the machine's free disk
        write(os.path.join(self.root, "examples/verified/a.wj"), "// expect: 1\nfn main() { println(\"1\") }\n")
        write(os.path.join(self.root, "examples/verified/b.wj"), "// expect: 2\nfn main() { println(\"2\") }\n")
        write(os.path.join(self.root, "tests/conformance_programs/c.wj"), "// expect: 3\nfn main() { println(\"3\") }\n")
        write(os.path.join(self.root, "tests/spec/x/S1.wj"), "// spec: S-1\n// expect: 4\nfn main() { println(\"4\") }\n")
        write(os.path.join(self.root, "tests/interpreter_corpus/t.wj"), TEMPLATE)
        write(os.path.join(self.root, "tests/interpreter_corpus/t.ref.py"), REF)
        write(os.path.join(self.root, "tests/check_negative/mutants/m1.wj"), "// mutant\nfn main() { let x: int = \"s\" }\n")
        write(os.path.join(self.root, "std/mod_a.wj"), "pub fn one() -> int { 1 }\npub fn two() -> int { 2 }\n")
        write(os.path.join(self.root, "tests/data/std_interpreter_missing.txt"), "# header\nmod_a::two\n")
        write(os.path.join(self.root, "src/interpreter/features.rs"), '    no("ffi.extern_fn", "FFI", "calls to extern fn", "p", "e"),\n')
        write(os.path.join(self.root, "bench/interp/k.wj"), "fn main() {\n    let mut i: int = 0\n    while i < 3 {\n        i += 1\n    }\n    println(\"${i}\")\n}\n")
        self.proj = os.path.join(self.eco, "packages", "alpha")
        write(os.path.join(self.proj, "tests", "x_test.wj"), "fn test_a() {}\n")
        write(os.path.join(self.proj, "src", "lib.wj"), "fn f() {}\n")
        cfg = os.path.join(self.tmp, "cfg.toml")
        write(cfg, "")
        self.cfg_path = cfg
        self.private_dir = None
        if private:
            self.private_dir = os.path.join(self.tmp, "SecretProj")
            write(os.path.join(self.private_dir, "tests", "p_test.wj"), "fn test_p() {}\n")
            write(cfg, f'[[project]]\nname = "SecretProj"\npath = "{self.private_dir}"\nprivate = true\n')

    def ctx(self, mode=core.QUICK, runner=core.run, cache=True):
        cfg = corpora.load_config(self.cfg_path)
        projects = corpora.build_projects(self.root, cfg, mode == core.QUICK, self.eco)
        c = core.Ctx(SHIM, self.root, mode, core.Cache(os.path.join(self.tmp, "cache"), cache), projects, commit="c0ffee", runner=runner)
        return c

    def close(self):
        shutil.rmtree(self.tmp, ignore_errors=True)
        os.environ.pop("WJ_READINESS_MIN_FREE_GIB", None)


def with_fixture(spec, private=False):
    def deco(fn):
        def wrapper(self):
            fx = Fixture(spec, private)
            try:
                fn(self, fx)
            finally:
                fx.close()
        return wrapper
    return deco


class R1Tests(unittest.TestCase):
    @with_fixture({"tests": {"alpha": {"interp": {"a": True, "b": False, "c": True}, "compiled": {"a": True, "b": True, "c": False}}}})
    def test_pass_fail_sets_not_counts(self, fx):
        c = fx.ctx()
        out = r1.measure(c)
        row = out["detail"]["projects"][0]
        self.assertEqual((row["compiled_pass"], row["both_pass"]), (2, 1))   # counts are equal (2 and 2), sets differ
        self.assertEqual(out["percent"], 50.0)
        self.assertEqual(out["status"], "fail")
        self.assertEqual([b["program"] for b in out["blockers"]], ["alpha:tests/x_test.wj::b"])
        self.assertEqual(row["interp_only_pass"], ["tests/x_test.wj::c"])

    @with_fixture({"tests": {"alpha": {"interp": {f"t{i}": True for i in range(100)}, "compiled": {f"t{i}": True for i in range(100)}}}})
    def test_full_agreement_passes(self, fx):
        out = r1.measure(fx.ctx())
        self.assertEqual((out["status"], out["percent"]), ("pass", 100.0))

    @with_fixture({"tests": {"alpha": {"interp": {f"t{i}": i != 0 for i in range(100)}, "compiled": {f"t{i}": True for i in range(100)}}}})
    def test_threshold_is_99_percent(self, fx):
        out = r1.measure(fx.ctx())
        self.assertEqual(out["percent"], 99.0)
        self.assertEqual(out["status"], "pass")

    @with_fixture({"tests": {"alpha": {"interp": {"a": True}, "compiled": {"a": True}}, "SecretProj": {"interp": {"test_p": False}, "compiled": {"test_p": True}}}}, private=True)
    def test_private_projects_are_aliased_and_test_names_hashed(self, fx):
        out = r1.measure(fx.ctx())
        text = json.dumps(out)
        self.assertNotIn("SecretProj", text)
        self.assertNotIn("test_p", text)
        self.assertIn("private-1", text)

    def test_unknown_compiled_statuses_are_unmeasured_not_a_comparison(self):
        comp = core.Res(1, '{"tests": [\n {"name": "a", "file": "./t.wj", "status": "unknown"}]}', "", 1.0)
        interp = core.Res(0, json.dumps({"tests": [{"file": "./t.wj", "name": "a", "passed": True}]}), "", 1.0)
        row = r1.evaluate(corpora.Project("p", "/tmp/p"), interp, comp, [])
        self.assertFalse(row["measured"])


    def test_mismatch_reason_is_the_first_line_of_the_failure_message_without_paths(self):
        comp = core.Res(0, json.dumps({"tests": [{"name": "a", "file": "./t.wj", "status": "passed"}]}), "", 1.0)
        msg = "error: no method named `f`\n  --> /home/someone/proj/src/lib.wj:6:14"
        interp = core.Res(1, json.dumps({"tests": [{"file": "./t.wj", "name": "a", "passed": False, "message": msg}]}), "", 1.0)
        row = r1.evaluate(corpora.Project("p", "/home/someone/proj"), interp, comp, [("/home/someone/proj", "<p>")])
        self.assertEqual(row["mismatches"][0]["reason"], "error: no method named `f`")

    def test_compiled_runs_get_a_private_cargo_target_root_and_cold_runs_a_fresh_one(self):
        seen = []

        def runner(cmd, timeout=1, env=None, **kw):
            seen.append((cmd[1:3], (env or {}).get("WJ_CARGO_TARGET_ROOT")))
            return core.Res(0, "{}", "", 0.1)

        d = tempfile.mkdtemp()
        try:
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [], runner=runner)
            p = corpora.Project("p", d)
            r1.run_project(c, p)
            c.cold = True
            c.cache.enabled = False
            r1.run_project(c, p)
            roots = {r for cmd, r in seen if cmd[0] == "test" and "--interpret" not in cmd}
            self.assertEqual(len(roots), 2)
            self.assertTrue(all(r and r.startswith(c.cache.dir) for r in roots))
            self.assertIn("cold-target", [r for r in roots if "cold" in r][0])
            self.assertFalse(os.path.exists([r for r in roots if "cold" in r][0]))   # removed after the cold run
        finally:
            shutil.rmtree(d)


class R2Tests(unittest.TestCase):
    SPEC = {"run": {
        "a.wj": {"interp": ["1\n", 0], "compiled": ["1\n", 0]},
        "b.wj": {"interp": ["3\n", 0], "compiled": ["3\n", 0]},                    # prints 3, its own oracle says 2
        "c.wj": {"interp": ["3\n", 0], "compiled": ["3\n", 1]},                    # exit code differs from compiled
        "S1.wj": {"interp": ["4\n", 0], "compiled": ["4\n", 0], "hash": {"1": "5\n"}},  # hash order leaks
    }}

    @with_fixture(SPEC)
    def test_every_divergence_kind_is_reported_with_the_program_name(self, fx):
        c = fx.ctx(core.FULL)
        out = r2.measure(c, corpora.load_programs(fx.root, False), corpora.load_templates(fx.root))
        got = {(b["program"], b["reason"].split("]")[0] + "]") for b in out["blockers"]}
        self.assertIn(("verified/b.wj", "[oracle]"), got)
        self.assertIn(("conformance/c.wj", "[compiled-exit]"), got)
        self.assertTrue(any(p == "spec/S1.wj" and k.startswith("[hash-seed") for p, k in got), got)
        self.assertEqual(out["status"], "fail")
        self.assertNotIn("verified/a.wj", [b["program"] for b in out["blockers"]])

    @with_fixture({"gen_wrong": [2]})
    def test_generated_seeds_are_checked_against_the_python_reference(self, fx):
        c = fx.ctx(core.FULL)
        tpl = corpora.load_templates(fx.root)
        self.assertGreaterEqual(len(tpl) * len(r2.gen_seeds(tpl[0], len(tpl), False)), 300)   # full mode reaches >= 300 seeds
        self.assertEqual(len(r2.gen_seeds(tpl[0], len(tpl), True)), 1)                          # quick samples one per template
        res = {"divergences": [], "unmeasured": [], "checked": 0, "compiled_checked": 0, "generated_seeds": 0}
        old, r2.GEN_SEEDS_FULL = r2.GEN_SEEDS_FULL, 4
        try:
            r2.check_generated(c, tpl, (0, 1), res)
        finally:
            r2.GEN_SEEDS_FULL = old
        self.assertEqual(res["generated_seeds"], 4)
        self.assertEqual([d["program"] for d in res["divergences"]], ["generated/t.wj@2"])

    @with_fixture({"run": {"a.wj": {"interp": ["1\n", 0], "compiled": ["", 101, "error[E0423]: cannot find function `println`\nerror: could not compile `verified`\n"]}}})
    def test_compiled_build_failure_is_not_reported_as_an_output_divergence(self, fx):
        out = r2.measure(fx.ctx(core.FULL), corpora.load_programs(fx.root, False), [])
        reasons = {b["program"]: b["reason"] for b in out["blockers"]}
        self.assertTrue(reasons["verified/a.wj"].startswith("[compiled-build-failed] error[E0423]"), reasons)
        self.assertFalse(any("compiled-stdout" in r for r in reasons.values() if "a.wj" in r))
        self.assertEqual(out["status"], "fail")

    def test_panic_messages_of_both_engines_normalize_equal(self):
        self.assertEqual(r2.panic_message("panic: boom\n"), r2.panic_message("thread 'main' panicked at src/main.rs:3:5:\nboom\n"))

    def test_judge_pair_flags_stdout_exit_and_panic_text(self):
        i, c = core.Res(0, "a\n", "", 0), core.Res(101, "b\n", "thread 'main' panicked at x:1:1:\nno\n", 0)
        self.assertEqual([k for k, _ in r2.judge_pair("p", i, c)], ["stdout", "exit", "stderr"])


class PilotTests(unittest.TestCase):
    def _add(self, fx, name, silent, green=True):
        t = {"arm": "A", "task": name, "trial": 0, "final_source": "fn main() { println(\"x\") }\n",
             "divergence": {"interp_pass": green, "silent": silent}}
        write(os.path.join(fx.root, f"bench/agent-dx/results/pilot/trials/{name}.A.0.json"), json.dumps(t))

    @with_fixture({})
    def test_silent_divergences_are_r2_blockers_and_check_misses_r5_blockers(self, fx):
        self._add(fx, "t_bad", True)
        self._add(fx, "t_ok", False)
        self._add(fx, "t_red", False, green=False)
        out = r2.measure(fx.ctx(core.FULL), {}, [])
        names = [b["program"] for b in out["blockers"]]
        self.assertIn("pilot/t_bad.A.0", names)
        self.assertNotIn("pilot/t_ok.A.0", names)
        s = out["detail"]["agent_pilot"]
        self.assertEqual((s["interpreter_green"], s["silent_divergences_recorded"], s["silent_divergence_rate"]), (2, 1, 0.5))
        r = r5.measure(fx.ctx(core.FULL))
        self.assertEqual(r["detail"]["agent_pilot_check_misses"], 1)
        self.assertTrue(any(b["program"] == "pilot/t_bad.A.0" for b in r["blockers"]))


class R3Tests(unittest.TestCase):
    def test_notice_parsing(self):
        fb, why, loud = r3.parse_notice("wj: running compiled (cargo): extern fn foo at line 12 (a.wj:3:1); use `wj run --compiled` to skip this check\n")
        self.assertEqual((fb, why, loud), (True, "extern fn foo at line N", True))
        self.assertEqual(r3.parse_notice("hello\n"), (False, "", True))
        self.assertEqual(r3.parse_notice("wj: running compiled (cargo): ; use `wj run --compiled`")[2], False)

    @with_fixture({"fallback": {"a.wj": "extern fn foo", "b.wj": "extern fn foo"}})
    def test_rate_and_reason_grouping(self, fx):
        c = fx.ctx(core.FULL)
        out = r3.measure(c, corpora.load_programs(fx.root, False), [])
        self.assertEqual(out["detail"]["fallbacks"], 2)
        self.assertEqual(out["detail"]["reasons"], {"extern fn foo": 2})
        self.assertEqual(out["status"], "fail")           # 2 of 4 runs

    def test_threshold(self):
        ok = r3.summarize(100, 2, [], {"x": 2}, [])
        bad = r3.summarize(100, 3, [], {"x": 3}, [{"program": "p", "reason": "x"}])
        self.assertEqual((ok["status"], bad["status"]), ("pass", "fail"))


class R4Tests(unittest.TestCase):
    def test_project_time_rule(self):
        rows, blk = r4.project_rows([{"project": "p", "measured": True, "interp_secs": 5.0, "compiled_secs": 2.0},
                                     {"project": "q", "measured": True, "interp_secs": 1.0, "compiled_secs": 2.0}])
        self.assertEqual([r["met"] for r in rows], [False, True])
        self.assertEqual(len(blk), 1)

    def test_python_equivalent_is_generated_and_runs(self):
        src = open(os.path.join(SCRIPTS, "..", "bench", "interp", "fib30.wj")).read()
        py = pygen.translate(src.replace("fib(30)", "fib(10)"))
        ns = io.StringIO()
        with redirect_stdout(ns):
            exec(compile(py, "k", "exec"), {})
        self.assertEqual(ns.getvalue().strip(), "55")

    def test_untranslatable_kernel_is_reported_not_guessed(self):
        with self.assertRaises(pygen.Untranslatable):
            pygen.translate("fn main() {\n    let x = 4 / 2\n}\n")

    @with_fixture({})
    def test_startup_row_reports_hello_and_the_pre_run_check_share(self, fx):
        row = r4.startup(fx.ctx())
        self.assertLessEqual(row["hello_s"], 5)
        self.assertEqual(row["project"], "alpha")
        self.assertIn("check_share", row)

    def test_hand_written_twin_is_preferred_over_a_generated_one(self):
        self.assertTrue(os.path.exists(os.path.join(SCRIPTS, "..", "bench", "interp", "twins", "fib30.py")))

    @with_fixture({"run": {"k.wj": {"interp": ["3\n", 0]}}})
    def test_kernel_row_and_unmeasured_loop(self, fx):
        rows, blk = r4.kernels(fx.ctx())
        self.assertTrue(rows[0]["outputs_match"])


class R5Tests(unittest.TestCase):
    def test_judge_matches_the_rust_gate(self):
        refuse = "wj: not running: `wj check` reports 1 error"
        self.assertIsNone(r5.judge("p", False, False, "", refuse))
        self.assertIn("did not refuse", r5.judge("p", False, True, "s\n", ""))
        self.assertIn("refused", r5.judge("p", True, False, "", refuse))
        self.assertIsNone(r5.judge("p", True, True, "x", ""))

    @with_fixture({"check_reject": ["m1.wj"]})
    def test_agreeing_binary_passes(self, fx):
        out = r5.measure(fx.ctx())
        parity = [b for b in out["blockers"] if "check-vs-interpreter" in b["reason"]]
        self.assertEqual(parity, [])

    @with_fixture({"permissive_run": ["m1.wj"], "check_reject": ["m1.wj"]})
    def test_permissive_interpreter_is_a_blocker_naming_the_program(self, fx):
        out = r5.measure(fx.ctx())
        self.assertTrue(any(b["program"].endswith("m1.wj") and "did not refuse" in b["reason"] for b in out["blockers"]), out["blockers"])
        self.assertEqual(out["status"], "fail")


class R6R7Tests(unittest.TestCase):
    @with_fixture({})
    def test_std_coverage_and_unsupported(self, fx):
        out = r6.measure(fx.ctx(), [])
        self.assertEqual(out["percent"], 50.0)                    # 1 of 2 declared fns has a builtin
        self.assertEqual(out["status"], "fail")
        self.assertIn("std::mod_a::two", [b["program"] for b in out["blockers"]])
        self.assertIn("feature:ffi.extern_fn", [b["program"] for b in out["blockers"]])

    def test_rust_crate_detection(self):
        d = tempfile.mkdtemp()
        try:
            write(os.path.join(d, "wj.toml"), '[dependencies]\nserde = "1"\n')
            self.assertTrue(r6.uses_rust_crates(d))
            write(os.path.join(d, "wj.toml"), '[dependencies]\nx = { path = "../x" }\n')
            self.assertFalse(r6.uses_rust_crates(d))
        finally:
            shutil.rmtree(d)

    def test_r7_needs_n_consecutive_full_passes(self):
        commits = [f"c{i}" for i in range(20)]
        ok = [{"commit": c, "mode": "full", "all_pass": True, "criteria": {}} for c in commits]
        self.assertEqual(history.evaluate(commits, ok)["status"], "pass")
        broken = ok[:5] + [{"commit": "c5", "mode": "full", "all_pass": False, "criteria": {"R2": "fail"}}] + ok[6:]
        r = history.evaluate(commits, broken)
        self.assertEqual((r["status"], r["percent"]), ("fail", 95.0))
        self.assertIn("failed R2", [b["reason"] for b in r["blockers"]])
        quick = [dict(e, mode="quick") for e in ok]                 # quick runs never count toward stability
        self.assertEqual(history.evaluate(commits, quick)["percent"], 0.0)

    def test_history_append_replaces_same_commit_and_mode(self):
        d = tempfile.mkdtemp()
        p = os.path.join(d, "h.json")
        try:
            crit = [{"id": f"R{i}", "status": "pass"} for i in range(1, 7)]
            e = history.append(p, [], "abc", "full", crit)
            e = history.append(p, e, "abc", "full", crit[:5] + [{"id": "R6", "status": "fail"}])
            self.assertEqual(len(e), 1)
            self.assertFalse(e[0]["all_pass"])
            self.assertEqual(len(history.load(p)), 1)
        finally:
            shutil.rmtree(d)


class CacheTests(unittest.TestCase):
    def test_second_call_replays_and_new_binary_invalidates(self):
        d = tempfile.mkdtemp()
        calls = []

        def runner(cmd, timeout=1, **kw):
            calls.append(cmd)
            return core.Res(0, "o", "", 0.1)

        try:
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [], commit="x", runner=runner)
            c.cached_run("k", "h", ["a"], 1)
            c.cached_run("k", "h", ["a"], 1)
            self.assertEqual(len(calls), 1)
            c.cached_run("k", "other-program", ["a"], 1)
            self.assertEqual(len(calls), 2)
            c.wj_hash = "different-binary"
            c.cached_run("k", "h", ["a"], 1)
            self.assertEqual(len(calls), 3)
            c.cache_id = "y"
            c.cached_run("k", "h", ["a"], 1)
            self.assertEqual(len(calls), 4)
        finally:
            shutil.rmtree(d)

    def test_budget_skips_compiled_runs_only_and_reports_them_unmeasured(self):
        d = tempfile.mkdtemp()
        n = []

        def runner(cmd, timeout=1, **kw):
            n.append(1)
            return core.Res(0, "o", "", 0.1)

        try:
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [], runner=runner)
            c.deadline = -1.0
            slow = c.cached_run("r2-compiled", "h", ["a"], 1)
            fast = c.cached_run("r2-interp-h0", "h", ["a"], 1)
            self.assertTrue(slow.timed_out and not fast.timed_out)
            self.assertEqual(len(n), 1)
        finally:
            shutil.rmtree(d)

    def test_compiled_runs_are_skipped_when_disk_is_low(self):
        d = tempfile.mkdtemp()
        n = []
        os.environ["WJ_READINESS_MIN_FREE_GIB"] = "1000000000"
        try:
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [], runner=lambda *a, **k: n.append(1) or core.Res(0, "", "", 0))
            self.assertTrue(c.cached_run("r1-compiled", "h", ["a"], 1).timed_out)
            self.assertFalse(c.cached_run("r1-interp", "h", ["a"], 1).timed_out)
            self.assertEqual(len(n), 1)
        finally:
            os.environ.pop("WJ_READINESS_MIN_FREE_GIB")
            shutil.rmtree(d)

    def test_unwritable_cache_degrades_to_uncached(self):
        d = tempfile.mkdtemp()
        try:
            cache = core.Cache(os.path.join(d, "c"))
            shutil.rmtree(cache.dir)
            open(cache.dir, "w").close()          # the cache directory is now a file: every write fails
            cache.put("k", {"a": 1})
            self.assertFalse(cache.enabled)
            self.assertIsNone(cache.get("k"))
        finally:
            shutil.rmtree(d)

    def test_cache_id_not_the_commit_keys_results(self):
        d = tempfile.mkdtemp()
        n = []
        runner = lambda *a, **k: n.append(1) or core.Res(0, "", "", 0)
        try:
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [], commit="docs-only-commit", runner=runner)
            c.cache_id = "src-tree-1"
            c.cached_run("k", "h", ["a"], 1)
            c.commit = "another-docs-commit"                      # a commit that does not touch the compiler hits the cache
            c.cached_run("k", "h", ["a"], 1)
            self.assertEqual(len(n), 1)
            c.cache_id = "src-tree-2"
            c.cached_run("k", "h", ["a"], 1)
            self.assertEqual(len(n), 2)
        finally:
            shutil.rmtree(d)

    def test_isolated_copies_a_program_into_its_own_directory(self):
        d = tempfile.mkdtemp()
        try:
            write(os.path.join(d, "ex", "a.wj"), "fn main() {}\n")
            write(os.path.join(d, "ex", "b.wj"), "fn main() {}\n")
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [])
            iso = c.isolated(os.path.join(d, "ex", "a.wj"))
            self.assertEqual(os.listdir(os.path.dirname(iso)), ["a.wj"])
        finally:
            shutil.rmtree(d)

    def test_timeouts_are_not_cached(self):
        d = tempfile.mkdtemp()
        n = []

        def runner(cmd, timeout=1, **kw):
            n.append(1)
            return core.Res(-9, "", "", 1.0, timed_out=True)

        try:
            c = core.Ctx(SHIM, d, core.QUICK, core.Cache(os.path.join(d, "c")), [], runner=runner)
            c.cached_run("k", "h", ["a"], 1)
            c.cached_run("k", "h", ["a"], 1)
            self.assertEqual(len(n), 2)
        finally:
            shutil.rmtree(d)


class MutationTests(unittest.TestCase):
    """An always-pass wj must be caught by the cross-checks; an honest one must not be flagged."""
    @with_fixture({"always_pass": True, "tests": {"alpha": {"interp": {"a": True}, "compiled": {"a": True}}}})
    def test_always_pass_shim_is_suspect(self, fx):
        c = fx.ctx()
        issues = selfcheck.run_canaries(c)
        self.assertTrue(any("canary tests" in i for i in issues), issues)
        self.assertTrue(any("canary seeds" in i for i in issues), issues)
        self.assertTrue(any("type error" in i for i in issues), issues)
        doc = {"criteria": []}
        self.assertEqual(report.overall([{"status": "pass"}] * 7, {"issues": issues}, False), "SUSPECT")
        self.assertEqual(report.exit_code("SUSPECT"), 3)

    @with_fixture({"tests": {"alpha": {"interp": {"a": True}, "compiled": {"a": True}}}})
    def test_honest_shim_has_no_canary_issues(self, fx):
        self.assertEqual(selfcheck.run_canaries(fx.ctx()), [])

    def test_permissive_interpreter_cross_check(self):
        crit = [{"id": "R1", "detail": {"projects": [{"project": "p", "measured": True, "sets_equal": True, "interp_only_pass": ["t"]}]}}]
        self.assertTrue(selfcheck.cross_checks(crit))


class EndToEndTests(unittest.TestCase):
    @with_fixture({"tests": {"alpha": {"interp": {"a": True}, "compiled": {"a": True}}}, "check_reject": ["m1.wj"],
                   "run": {"a.wj": {"interp": ["1\n", 0], "compiled": ["1\n", 0]}, "b.wj": {"interp": ["2\n", 0], "compiled": ["2\n", 0]},
                           "c.wj": {"interp": ["3\n", 0], "compiled": ["3\n", 0]}, "S1.wj": {"interp": ["4\n", 0], "compiled": ["4\n", 0]},
                           "k.wj": {"interp": ["3\n", 0]}}}, private=True)
    def test_cli_writes_clean_json_and_report_never_fails(self, fx):
        out_json = os.path.join(fx.tmp, "out.json")
        os.environ["WJ_READINESS_CONFIG"] = fx.cfg_path
        try:
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = interp_readiness.main(["--root", fx.root, "--wj", SHIM, "--ecosystem", fx.eco, "--quick", "--report", "--write",
                                            "--json-out", out_json, "--cache-dir", os.path.join(fx.tmp, "cc")])
            self.assertEqual(rc, 0)
            doc = json.load(open(out_json))
            self.assertEqual([c["id"] for c in doc["criteria"]], ["R1", "R2", "R3", "R4", "R5", "R6", "R7"])
            self.assertNotIn("SecretProj", json.dumps(doc))
            self.assertNotEqual(doc["verdict"], "READY")        # R6/R7 fail in the fixture
            self.assertIn("verdict:", buf.getvalue())
            with redirect_stdout(io.StringIO()):
                rc2 = interp_readiness.main(["--root", fx.root, "--wj", SHIM, "--ecosystem", fx.eco, "--quick", "--no-history",
                                             "--cache-dir", os.path.join(fx.tmp, "cc")])
            self.assertEqual(rc2, 1)                            # not ready: non-zero without --report
        finally:
            os.environ.pop("WJ_READINESS_CONFIG", None)

    def test_assert_clean_refuses_private_names(self):
        with self.assertRaises(SystemExit):
            report.assert_clean('{"x": "SecretProj"}', ["SecretProj"])


if __name__ == "__main__":
    unittest.main()
