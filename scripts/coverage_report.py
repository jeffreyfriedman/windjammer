#!/usr/bin/env python3
"""Aggregate an llvm-cov JSON export per src/ module and compare with / update the baseline.

Ratchet rule: per module, covered-line percentage may not drop by more than TOLERANCE points
(new code with tests keeps it level; deleting tests or adding untested code lowers it).
"""
import json
import sys
from pathlib import Path

TOLERANCE = 0.5
USER_FACING = ("src/bin/", "src/cli/", "src/test_runner/", "src/linter/", "src/formatter/", "src/check/", "src/interpreter/")


def module_of(rel: str) -> str:
    parts = rel.split("/")
    return parts[1] if len(parts) > 2 else parts[1].removesuffix(".rs")


def aggregate(export: dict, root: Path) -> dict:
    mods, files = {}, []
    for f in export["data"][0]["files"]:
        try:
            rel = str(Path(f["filename"]).resolve().relative_to(root))
        except ValueError:
            continue
        if not rel.startswith("src/"):
            continue
        s = f["summary"]
        lines, br = s["lines"], s.get("branches", {"count": 0, "covered": 0})
        m = mods.setdefault(module_of(rel), {"lines": 0, "covered": 0, "branches": 0, "branches_covered": 0})
        m["lines"] += lines["count"]
        m["covered"] += lines["covered"]
        m["branches"] += br["count"]
        m["branches_covered"] += br["covered"]
        files.append((rel, lines["count"] - lines["covered"], lines["count"]))
    for m in mods.values():
        m["pct"] = round(100.0 * m["covered"] / m["lines"], 2) if m["lines"] else 100.0
    tot_l = sum(m["lines"] for m in mods.values())
    tot_c = sum(m["covered"] for m in mods.values())
    uncovered = sorted((f for f in files if f[0].startswith(USER_FACING)), key=lambda f: -f[1])[:15]
    return {
        "total": {"lines": tot_l, "covered": tot_c, "pct": round(100.0 * tot_c / max(tot_l, 1), 2)},
        "modules": dict(sorted(mods.items())),
        "biggest_uncovered_user_facing": [{"file": f, "uncovered_lines": u, "lines": n} for f, u, n in uncovered],
    }


def main() -> int:
    mode, export_path, baseline_path, scope = sys.argv[1:5]
    root = Path.cwd().resolve()
    got = aggregate(json.loads(Path(export_path).read_text()), root)
    got["scope"] = scope
    base_file = Path(baseline_path)
    base = json.loads(base_file.read_text()) if base_file.exists() else None
    print(f"coverage ({scope}): {got['total']['covered']}/{got['total']['lines']} lines = {got['total']['pct']}%")
    for u in got["biggest_uncovered_user_facing"][:10]:
        print(f"  uncovered {u['uncovered_lines']:5} / {u['lines']:5}  {u['file']}")
    if mode == "--update" or base is None:
        base_file.parent.mkdir(parents=True, exist_ok=True)
        base_file.write_text(json.dumps(got, indent=2) + "\n")
        print(f"wrote {baseline_path}")
        return 0
    if base.get("scope") != scope:
        print(f"baseline scope {base.get('scope')!r} != {scope!r}: compare like with like")
        return 1
    drops = []
    for name, b in base["modules"].items():
        g = got["modules"].get(name)
        if g and g["pct"] + TOLERANCE < b["pct"]:
            drops.append(f"{name}: {b['pct']}% -> {g['pct']}%")
    if drops:
        print("coverage dropped:\n  " + "\n  ".join(drops))
        return 1
    print("coverage ratchet: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
