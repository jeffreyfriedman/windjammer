"""Oracle generation: run each task's independent Python reference over seeded cases.

tasks/<id>/oracle/ref.py must define:
  SAMPLES : list of case dicts shown to the agent (also in hidden vectors)
  EDGE    : list of hand-picked edge cases
  rand(rng) -> case dict          (rng is random.Random seeded from the task id)
  solve(case) -> stdout str | (stdout str, exit_code)
A case is {"stdin": str, "args": [str], "files": {name: content}}.
"""
import importlib.util
import json
import os
import random
import zlib

from .paths import TASKS

N_RANDOM = 30


def load_ref(task_id):
    path = os.path.join(TASKS, task_id, "oracle", "ref.py")
    spec = importlib.util.spec_from_file_location(f"ref_{task_id}", path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def _vec(ref, case):
    out = ref.solve(json.loads(json.dumps(case)))
    code = 0
    if isinstance(out, tuple):
        out, code = out
    v = {"stdin": case.get("stdin", ""), "args": case.get("args", []), "files": case.get("files", {}),
         "stdout": out}
    if code:
        v["code"] = code
    return v


def build_vectors(task_id):
    ref = load_ref(task_id)
    rng = random.Random(zlib.crc32(task_id.encode()))
    cases = list(ref.SAMPLES) + list(ref.EDGE)
    for _ in range(N_RANDOM):
        cases.append(ref.rand(rng))
    seen, vectors = set(), []
    for c in cases:
        key = json.dumps(c, sort_keys=True)
        if key in seen:
            continue
        seen.add(key)
        vectors.append(_vec(ref, c))
    samples = [_vec(ref, c) for c in ref.SAMPLES]
    return samples, vectors


def write(task_id):
    samples, vectors = build_vectors(task_id)
    d = os.path.join(TASKS, task_id)
    os.makedirs(os.path.join(d, "oracle"), exist_ok=True)
    with open(os.path.join(d, "samples.json"), "w") as f:
        json.dump(samples, f, indent=1, sort_keys=True)
        f.write("\n")
    with open(os.path.join(d, "oracle", "vectors.json"), "w") as f:
        json.dump(vectors, f, indent=1, sort_keys=True)
        f.write("\n")
    return len(samples), len(vectors)
