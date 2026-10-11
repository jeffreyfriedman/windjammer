"""Command line: python3 -m agentdx <command>."""
import argparse
import json
import sys

from . import baseline, gen, tasks as T, toolchains
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
