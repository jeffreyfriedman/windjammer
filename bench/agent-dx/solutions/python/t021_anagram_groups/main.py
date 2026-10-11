import sys

words = set(w.lower() for w in sys.stdin.read().split())
groups = {}
for w in words:
    groups.setdefault("".join(sorted(w)), []).append(w)
lines = sorted(sorted(g) for g in groups.values() if len(g) >= 2)
if not lines:
    print("none")
for g in lines:
    print(" ".join(g))
