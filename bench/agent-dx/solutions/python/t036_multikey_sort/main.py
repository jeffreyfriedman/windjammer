import sys

lines = sys.stdin.read().split("\n")
spec = [s.split(":") for s in lines[0].strip().split(",")]
recs = [l.split() for l in lines[1:] if l.strip()]
IDX = {"name": 0, "age": 1, "city": 2}
for field, d in reversed(spec):
    i = IDX[field]
    recs.sort(key=lambda r: int(r[i]) if field == "age" else r[i].encode(), reverse=(d == "desc"))
for r in recs:
    print(" ".join(r))
