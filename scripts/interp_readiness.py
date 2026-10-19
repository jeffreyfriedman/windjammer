#!/usr/bin/env python3
"""Interpreter-primary readiness scorecard: measures R1-R7 of docs/design/interpreter-primary-readiness.md.

  scripts/interp_readiness.py --quick            small subsets (minutes), exit 1 unless every criterion passes
  scripts/interp_readiness.py --full --write     everything; writes docs/generated/interp_readiness.json
  scripts/interp_readiness.py --report           print the scorecard, always exit 0

Private projects come from an untracked config (~/.config/wj-readiness.toml or $WJ_READINESS_CONFIG /
$WJ_READINESS_PROJECTS); their names and paths never reach the output. Builds run serially; results are
cached by commit + wj binary + program hash (.wj-readiness-cache/), so an interrupted run resumes.
Exit: 0 ready, 1 not ready, 3 suspect measurement (cross-checks failed).
"""
import argparse, json, os, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from readiness import core, corpora, history, r1, r2, r3, r4, r5, r6, report, selfcheck  # noqa: E402

ROOT = os.path.dirname(HERE)


def default_wj():
    for c in (os.environ.get("WJ_BIN"), os.path.join(ROOT, "target/release/wj"), os.path.join(ROOT, "target/debug/wj")):
        if c and os.path.exists(c):
            return c
    return "wj"


def git(root, *a):
    r = subprocess.run(["git", *a], cwd=root, capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else ""


def main(argv=None, runner=core.run):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--quick", action="store_true", help="small subsets (default)")
    g.add_argument("--full", action="store_true", help="every program, >= 300 generated seeds, hash seeds 0..3")
    ap.add_argument("--report", action="store_true", help="print the scorecard and exit 0")
    ap.add_argument("--write", action="store_true", help="write docs/generated/interp_readiness.json")
    ap.add_argument("--wj", default=default_wj(), help="wj binary (default: $WJ_BIN, target/release/wj, target/debug/wj)")
    ap.add_argument("--ecosystem", help="windjammer-ecosystem checkout (default: config, then ../windjammer-ecosystem)")
    ap.add_argument("--config", help="untracked config listing extra projects")
    ap.add_argument("--root", default=ROOT, help="compiler checkout to measure (default: this one)")
    ap.add_argument("--cache-dir")
    ap.add_argument("--no-cache", action="store_true")
    ap.add_argument("--history")
    ap.add_argument("--no-history", action="store_true", help="do not append this run to the history file")
    ap.add_argument("--cold", action="store_true", help="compiled runs start from an empty cargo target dir (default with --full)")
    ap.add_argument("--warm", action="store_true", help="compiled runs share one private target dir (default with --quick)")
    ap.add_argument("--cargo-gates", action="store_true", help="also run the real cargo gates for R5 (heavy)")
    ap.add_argument("--only", help="comma list of criteria to measure, e.g. R2,R5 (others reported unmeasured)")
    ap.add_argument("--json-out")
    a = ap.parse_args(argv)
    root = os.path.abspath(a.root)
    a.cache_dir = a.cache_dir or os.path.join(root, ".wj-readiness-cache")
    a.history = a.history or os.path.join(root, ".wj-readiness-history.json")
    a.json_out = a.json_out or os.path.join(root, "docs/generated/interp_readiness.json")
    mode = core.FULL if a.full else core.QUICK
    cfg = corpora.load_config(a.config)
    projects = corpora.build_projects(root, cfg, mode == core.QUICK, a.ecosystem)
    if not projects:
        print("warning: no projects found (set --ecosystem or the config); R1/R3/R4/R6 will be unmeasured", file=sys.stderr)
    commit = git(root, "rev-parse", "HEAD") or "unknown"
    ctx = core.Ctx(os.path.abspath(a.wj) if os.sep in a.wj else a.wj, root, mode, core.Cache(a.cache_dir, not a.no_cache),
                   projects, commit=commit, runner=runner, log=lambda m: print("  " + m, file=sys.stderr))
    ctx.cold = a.cold or (a.full and not a.warm)
    want = set(a.only.split(",")) if a.only else set(report.ORDER)
    unmeasured = lambda i, t: {"id": i, "title": t, "target": "", "percent": None, "status": "unmeasured", "detail": {}, "blockers": []}
    corp, templates = corpora.load_programs(root, ctx.quick), corpora.load_templates(root)
    crit = {}
    crit["R1"] = r1.measure(ctx) if "R1" in want else unmeasured("R1", "Test suites pass interpreted")
    rows = crit["R1"]["detail"].get("projects", [])
    crit["R2"] = r2.measure(ctx, corp, templates) if "R2" in want else unmeasured("R2", "No silent divergence")
    crit["R3"] = r3.measure(ctx, corp, rows) if "R3" in want else unmeasured("R3", "Fallback is rare and loud")
    crit["R4"] = r4.measure(ctx, rows) if "R4" in want else unmeasured("R4", "Fast enough")
    crit["R5"] = r5.measure(ctx, a.cargo_gates) if "R5" in want else unmeasured("R5", "Diagnostics agree")
    crit["R6"] = r6.measure(ctx, rows) if "R6" in want else unmeasured("R6", "Stdlib and project coverage")
    first6 = [crit[k] for k in report.ORDER[:6]]
    hist = history.load(a.history)
    if not a.no_history:
        hist = history.append(a.history, hist, commit, mode, first6)
    crit["R7"] = history.evaluate(history.last_commits(root, runner=runner), hist) if "R7" in want else unmeasured("R7", "Stability")
    criteria = [crit[k] for k in report.ORDER]
    sc = selfcheck.run_all(ctx, criteria)
    profile = "debug" if "/debug/" in ctx.wj else ("release" if "/release/" in ctx.wj else "unknown")
    doc = report.build_doc(commit, mode, round(os.getloadavg()[0], 1), profile, criteria, sc, ctx.quick,
                           time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
    print(report.render(doc))
    if a.write:
        text = json.dumps(doc, indent=1) + "\n"
        forbidden = [p.path for p in projects if p.private] + [p.name for p in projects if p.private] + [os.path.expanduser("~")]
        report.assert_clean(text, forbidden)
        os.makedirs(os.path.dirname(a.json_out), exist_ok=True)
        open(a.json_out, "w").write(text)
    return 0 if a.report else report.exit_code(doc["verdict"])


if __name__ == "__main__":
    sys.exit(main())
