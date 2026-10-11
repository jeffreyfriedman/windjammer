import sys
import re
from collections import Counter

pat = re.compile(r"([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}) (DEBUG|INFO|WARN|ERROR) ([^ ]+)(?: (.*))?", re.S)
lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
levels = Counter()
comps = Counter()
stamps = []
errors = []
bad = 0
for line in lines:
    m = pat.fullmatch(line)
    if not m:
        bad += 1
        continue
    ts, lv, comp, msg = m.group(1), m.group(2), m.group(3), m.group(4) or ""
    levels[lv] += 1
    comps[comp] += 1
    stamps.append(ts)
    if lv == "ERROR":
        errors.append("  [%s] %s" % (comp, msg))
print("levels: " + " ".join("%s=%d" % (l, levels[l]) for l in ("DEBUG", "INFO", "WARN", "ERROR")))
print("top components:")
for c, n in sorted(comps.items(), key=lambda kv: (-kv[1], kv[0]))[:3]:
    print("  %s %d" % (c, n))
print("first: " + (min(stamps) if stamps else "-"))
print("last: " + (max(stamps) if stamps else "-"))
print("errors:")
for e in errors:
    print(e)
print("malformed: %d" % bad)
