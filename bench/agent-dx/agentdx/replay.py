"""Deterministic replay of recorded transcripts: no LLM, same grading code path."""
import json
import os

from . import driver, metrics, workspace as W


class ReplayAgent:
    """Writes the recorded source of iteration i into the workspace."""

    def __init__(self, transcript):
        self.t = transcript

    def turn(self, wd, iteration, prompt_file, feedback_file, lang, task):
        rec = self.t["iterations"][iteration - 1]
        with open(os.path.join(wd, W.src_name(lang)), "w") as f:
            f.write(rec["source"])
        return {"usage": rec.get("usage"), "exit": rec.get("agent_exit", 0), "log": "replay"}


def load(path):
    with open(path) as f:
        return json.load(f)


def replay(transcript):
    """Re-run grading on recorded sources. Returns (new_transcript, diffs)."""
    agent = ReplayAgent(transcript)
    new = driver.run_trial(transcript["task"], transcript["lang"], agent, transcript["max_iters"], transcript["trial"])
    diffs = []
    if len(new["iterations"]) != len(transcript["iterations"]):
        diffs.append(f"iteration count {len(transcript['iterations'])} -> {len(new['iterations'])}")
    for a, b in zip(transcript["iterations"], new["iterations"]):
        for key in ("status", "passed", "total"):
            if a["hidden"][key] != b["hidden"][key]:
                diffs.append(f"iteration {a['index']} hidden.{key}: recorded {a['hidden'][key]} replayed {b['hidden'][key]}")
        if a["build"]["ok"] != b["build"]["ok"]:
            diffs.append(f"iteration {a['index']} build.ok differs")
    return new, diffs


def recorded_metrics(transcript):
    """Metrics from recorded data only (no toolchains needed): the CI path."""
    return metrics.trial_metrics(transcript)
