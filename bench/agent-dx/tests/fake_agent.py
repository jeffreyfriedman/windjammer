"""A scripted 'agent' for harness self-tests (python tasks). Modes:
  good      : copy the reference python solution
  syntax_then_good : iteration 1 writes a syntax error, later iterations the solution
  wrong_then_good  : iteration 1 writes a program with a wrong answer, later the solution
  checking  : runs ./check first (on an empty file), then writes the solution and runs ./check again
"""
import argparse
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ap = argparse.ArgumentParser()
ap.add_argument("--mode", required=True)
ap.add_argument("--task", required=True)
ap.add_argument("--iteration", type=int, required=True)
ap.add_argument("--workspace", required=True)
a = ap.parse_args()
dst = os.path.join(a.workspace, "main.py")
good = os.path.join(ROOT, "solutions", "python", a.task, "main.py")


def write(text):
    with open(dst, "w") as f:
        f.write(text)


if a.mode == "good":
    shutil.copy(good, dst)
elif a.mode == "syntax_then_good":
    write("def main(:\n    pass\n") if a.iteration == 1 else shutil.copy(good, dst)
elif a.mode == "wrong_then_good":
    write("print('0')\n") if a.iteration == 1 else shutil.copy(good, dst)
elif a.mode == "checking":
    write("def broken(:\n")
    subprocess.run(["./check"], cwd=a.workspace, capture_output=True)
    shutil.copy(good, dst)
    subprocess.run(["./check"], cwd=a.workspace, capture_output=True)
else:
    sys.exit("bad mode")
usage = os.environ.get("AGENTDX_USAGE_FILE")
if usage:
    import json
    json.dump({"input_tokens": 1000 * a.iteration, "output_tokens": 100 * a.iteration}, open(usage, "w"))
