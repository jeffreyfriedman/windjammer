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


# ---- pilot 4: loop-mode comparison with one sheet. The loop instruction comes from the prompt only, so the sheet's own loop
# bullet is removed from every arm; D additionally uses the v2 sheet (types, basics, input recipe, len cast).
P4_DIR = os.path.join(PACKS_DIR, "pilot4")
V2_KEEP = ("- Types:", "- No semicolons", "- Input:", "- `len()` is `usize`")


def _bullets(text):
    head, bullets, tail, seen_bullet = [], [], [], False
    for ln in text.split("\n"):
        if ln.startswith("- "):
            bullets.append(ln); seen_bullet = True
        elif not seen_bullet:
            head.append(ln)
        else:
            tail.append(ln)
    return head, bullets, tail


def p4_render():
    sheet = open(PACK_PATH).read()
    head, bullets, tail = _bullets(sheet)
    noloop = [b for b in bullets if not b.startswith("- `wj check FILE`")]
    if len(noloop) != len(bullets) - 1:
        raise ValueError("loop bullet not found exactly once")
    v2 = [b for b in noloop if b.startswith(V2_KEEP)]
    if len(v2) != len(V2_KEEP):
        raise ValueError("v2 bullet groups missing")
    return {"sheet_noloop.md": "\n".join(head + noloop + tail),
            "sheet_v2.md": "\n".join(head + v2 + [""])}


def p4_write():
    os.makedirs(P4_DIR, exist_ok=True)
    for name, text in p4_render().items():
        with open(os.path.join(P4_DIR, name), "w") as f:
            f.write(text)
