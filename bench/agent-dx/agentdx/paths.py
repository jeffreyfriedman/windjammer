"""Filesystem layout constants."""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TASKS = os.path.join(ROOT, "tasks")
SOLUTIONS = os.path.join(ROOT, "solutions")
TRANSCRIPTS = os.path.join(ROOT, "transcripts")
RESULTS = os.path.join(ROOT, "results")

LANGS = ["wj-interp", "wj", "rust", "go", "python", "ts"]
# language -> (solution dir, file name)
SOLUTION_FILE = {
    "wj-interp": ("wj", "main.wj"),
    "wj": ("wj", "main.wj"),
    "rust": ("rust", "main.rs"),
    "go": ("go", "main.go"),
    "python": ("python", "main.py"),
    "ts": ("ts", "main.ts"),
}


def solution_path(lang, task_id):
    """Reference solution for (lang, task): solutions/<dir>/<task_id>/<file>."""
    d, f = SOLUTION_FILE[lang]
    return os.path.join(SOLUTIONS, d, task_id, f)
