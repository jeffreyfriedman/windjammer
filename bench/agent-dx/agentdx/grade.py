"""Grade a workspace (one candidate solution) against vectors."""
from . import compare
from . import tasks as T
from .runners import classify_run_failure, get_runner


def run_vectors(lang, runner, build, wd, vectors, stop_after_fail=False, keep_failures=3):
    passed, failures, diag = 0, [], []
    for i, v in enumerate(vectors):
        rr = runner.run(build, v, wd)
        want_code = v.get("code", 0)
        ok = (not rr.timed_out) and rr.code == want_code and compare.same(v["stdout"], rr.stdout)
        if ok:
            passed += 1
            continue
        d = classify_run_failure(lang, rr) if (rr.timed_out or rr.code != want_code) else None
        if d is None:
            d = {"code": "", "message": "wrong output", "category": "wrong-output"}
        diag.append(d)
        if len(failures) < keep_failures:
            failures.append({"index": i, "category": d["category"], "message": d["message"][:300],
                             "stdin": v.get("stdin", "")[:300], "args": v.get("args", []),
                             "expected": v["stdout"][:300], "actual": rr.stdout[:300], "exit_code": rr.code})
        if stop_after_fail:
            break
    return passed, failures, diag


def grade(lang, task_id, wd, hidden=True, build=None):
    """Return a result dict. `hidden=True` uses oracle vectors, else visible samples."""
    runner = get_runner(lang)
    vectors = T.load_vectors(task_id) if hidden else T.load_samples(task_id)
    b = build or runner.build(wd)
    res = {"lang": lang, "task": task_id, "hidden": hidden, "build": b.to_dict(),
           "total": len(vectors), "passed": 0, "failures": [], "categories": []}
    if not b.ok:
        res["status"] = "build_error"
        res["categories"] = sorted({d["category"] for d in b.diagnostics} or {"other"})
        return res
    passed, failures, diag = run_vectors(lang, runner, b, wd, vectors)
    res["passed"], res["failures"] = passed, failures
    res["categories"] = sorted({d["category"] for d in diag})
    res["status"] = "pass" if passed == len(vectors) else "fail"
    return res
