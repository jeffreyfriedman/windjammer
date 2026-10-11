import sys

lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
if "---" in lines:
    idx = lines.index("---")
    ini, queries = lines[:idx], lines[idx + 1:]
else:
    ini, queries = lines, []
data = {"": {}}
cur = ""
for raw in ini:
    line = raw.strip()
    if not line or line[0] in ";#":
        continue
    if line.startswith("[") and line.endswith("]"):
        cur = line[1:-1]
        data.setdefault(cur, {})
    elif "=" in line:
        k, _, v = line.partition("=")
        data[cur][k.strip()] = v.strip()
for q in queries:
    if "." in q:
        sec, _, key = q.partition(".")
    else:
        sec, key = "", q
    print(data.get(sec, {}).get(key, "<none>"))
