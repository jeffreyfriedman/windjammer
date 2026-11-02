"""Tip ablation: derive packs from the default sheet (docs/agent/WINDJAMMER_FOR_AGENTS.md) by dropping one tip group.

A tip group is one bullet of the sheet, found by its leading text. `write_all` renders packs/ablation/*.md; a test keeps the
committed files equal to a fresh render, so an arm can never run a stale pack."""
import os

from .arms import PACKS_DIR, PACK_PATH
from .paths import ROOT

# arm id -> (group name, bullet prefix)
GROUPS = {
    "Sa": ("strings", "- Strings:"),
    "Sb": ("ownership", "- Ownership is inferred"),
    "Sc": ("input", "- Input:"),
    "Sd": ("len", "- `len()` is `usize`"),
    "Se": ("printing", "- Printing `Option`"),
    "Sf": ("tools", "- Unsure?"),
}
OUT_DIR = os.path.join(PACKS_DIR, "ablation")


def drop(text, prefix):
    lines = text.split("\n")
    hit = [i for i, l in enumerate(lines) if l.startswith(prefix)]
    if len(hit) != 1:
        raise ValueError(f"tip group {prefix!r} matches {len(hit)} lines")
    del lines[hit[0]]
    return "\n".join(lines)


def render():
    sheet = open(PACK_PATH).read()
    out = {"sheet_full.md": sheet}
    for arm, (name, prefix) in GROUPS.items():
        out[f"sheet_minus_{name}.md"] = drop(sheet, prefix)
    return out


def write_all():
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, text in render().items():
        with open(os.path.join(OUT_DIR, name), "w") as f:
            f.write(text)
