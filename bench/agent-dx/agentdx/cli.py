"""Command line: python3 -m agentdx <command>."""
import argparse
import json
import sys

from . import baseline, driver, gen, replay as R, report as RP, tasks as T, toolchains, workspace as W
from .paths import LANGS


def _tasks(arg):
    ids = T.list_task_ids()
    if not arg:
        return ids
    pats = arg.split(",")
    return [i for i in ids if any(i == p or i.startswith(p) for p in pats)]


def _langs(arg, tc):
    want = arg.split(",") if arg else list(LANGS)
    return toolchains.announce(tc, want)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="agentdx")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("toolchains")
    g = sub.add_parser("gen")
    g.add_argument("--tasks")
    b = sub.add_parser("baseline", help="grade reference solutions")
    b.add_argument("--tasks")
    b.add_argument("--langs")
    b.add_argument("--json", default=None)
    c = sub.add_parser("check-workspace", help="used by the agent: build + visible examples")
    c.add_argument("--workspace", required=True)
    c.add_argument("--lang", required=True)
    c.add_argument("--task", required=True)
    c.add_argument("--log", required=True)
    r = sub.add_parser("run", help="agent-in-the-loop trials")
    r.add_argument("--agent-cmd", required=True, help="shell template; see README")
    r.add_argument("--tasks")
    r.add_argument("--langs")
    r.add_argument("--max-iters", type=int, default=3)
    r.add_argument("--trials", type=int, default=1)
    r.add_argument("--out", default="results/transcripts")
    p = sub.add_parser("replay", help="re-grade recorded transcripts deterministically (no LLM)")
    p.add_argument("--transcripts", required=True)
    p.add_argument("--no-regrade", action="store_true", help="use recorded results only (no toolchains)")
    rp = sub.add_parser("report")
    rp.add_argument("--baseline")
    rp.add_argument("--transcripts")
    rp.add_argument("--md", default=None)
    rp.add_argument("--json", default=None)
    a = ap.parse_args(argv)
    if a.cmd == "toolchains":
        print(json.dumps(toolchains.detect(), indent=1))
    elif a.cmd == "gen":
        for t in _tasks(a.tasks):
            print(t, gen.write(t))
    elif a.cmd == "baseline":
        tc = toolchains.detect()
        res = baseline.run_matrix(_langs(a.langs, tc), _tasks(a.tasks))
        if a.json:
            with open(a.json, "w") as f:
                json.dump({"toolchains": tc, "results": res}, f, indent=1)
    elif a.cmd == "check-workspace":
        return W.check(a.task, a.lang, a.workspace, a.log)
    elif a.cmd == "run":
        tc = toolchains.detect()
        agent = driver.ShellAgent(a.agent_cmd)
        for lang in _langs(a.langs, tc):
            for tid in _tasks(a.tasks):
                for k in range(a.trials):
                    tr = driver.run_trial(tid, lang, agent, a.max_iters, k)
                    path = driver.save(tr, a.out)
                    m = tr["metrics"]
                    print(f"{tid} {lang} trial={k} green={m['green']} iters={m['iterations_used']} -> {path}", flush=True)
    elif a.cmd == "replay":
        bad = 0
        for t in RP.load_transcripts(a.transcripts):
            if a.no_regrade:
                m = R.recorded_metrics(t)
                print(t["task"], t["lang"], "green" if m["green"] else "red", "iters", m["iterations_used"])
                continue
            tc = toolchains.detect()
            if not toolchains.announce(tc, [t["lang"]]):
                continue
            _, diffs = R.replay(t)
            print(t["task"], t["lang"], "REPLAY-OK" if not diffs else "REPLAY-DIFF " + "; ".join(diffs))
            bad += bool(diffs)
        return 1 if bad else 0
    elif a.cmd == "report":
        base = json.load(open(a.baseline)) if a.baseline else None
        trs = RP.load_transcripts(a.transcripts) if a.transcripts else []
        md, js = RP.build_report(base, trs)
        if a.md:
            open(a.md, "w").write(md + "\n")
        else:
            print(md)
        if a.json:
            json.dump(js, open(a.json, "w"), indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
