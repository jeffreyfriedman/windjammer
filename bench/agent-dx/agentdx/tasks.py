"""Task loading: spec, visible samples, hidden vectors."""
import re
import json
import os

from .paths import TASKS


def list_task_ids():
    return sorted(d for d in os.listdir(TASKS)
                  if os.path.isfile(os.path.join(TASKS, d, "task.json")))


def load_task(task_id):
    d = os.path.join(TASKS, task_id)
    with open(os.path.join(d, "task.json")) as f:
        meta = json.load(f)
    with open(os.path.join(d, "spec.md")) as f:
        meta["spec"] = f.read()
    meta["id"] = task_id
    meta["dir"] = d
    return meta


def load_samples(task_id):
    with open(os.path.join(TASKS, task_id, "samples.json")) as f:
        return json.load(f)


def load_vectors(task_id):
    """HIDDEN vectors. Never copied into an agent workspace."""
    with open(os.path.join(TASKS, task_id, "oracle", "vectors.json")) as f:
        return json.load(f)


def render_prompt(task_id, lang, agent_mode=False):
    """Natural-language prompt given to the agent (spec + visible samples only)."""
    t = load_task(task_id)
    samples = load_samples(task_id)
    out = [f"# Task: {t['title']}", "", t["spec"].strip(), "", "## Examples", ""]
    for i, s in enumerate(samples, 1):
        out.append(f"### Example {i}")
        if s.get("args"):
            out.append("Command-line arguments: " + " ".join(s["args"]))
        for name, content in sorted(s.get("files", {}).items()):
            out += [f"File `{name}`:", "```", content.rstrip("\n"), "```"]
        out += ["Standard input:", "```", s.get("stdin", "").rstrip("\n"), "```",
                "Expected standard output:", "```", s["stdout"].rstrip("\n"), "```", ""]
    if agent_mode and any(re.search(r" =\S", x) for x in [t["spec"]] + [s.get("stdin", "") for s in samples]):
        # the Bash safety check refuses a space followed by `=` anywhere in a command, quoted or not (measured, pilot 8)
        out += ["Input for this task contains a space followed by `=`, which a shell command here cannot contain: write the input to a file "
                "with the Write tool and pass it with `--stdin-file FILE`.", ""]
    return "\n".join(out)
