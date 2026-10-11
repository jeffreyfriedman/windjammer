import sys


def fail(msg, code):
    print(msg)
    sys.exit(code)


if len(sys.argv) != 4:
    fail("usage: csv FILE COLUMN OP", 2)
fname, col, op = sys.argv[1:]
try:
    with open(fname, "rb") as f:
        text = f.read().decode("latin-1")
except OSError:
    fail("error: cannot read " + fname, 1)
lines = text.split("\n")
if lines and lines[-1] == "":
    lines.pop()
header = lines[0].split(",") if lines else []
if col not in header:
    fail("error: no such column " + col, 2)
if op not in ("sum", "min", "max", "count", "mean"):
    fail("error: unknown op " + op, 2)
idx = header.index(col)
rows = []
for n, line in enumerate(lines[1:], 2):
    f = line.split(",")
    rows.append((n, f[idx] if idx < len(f) else ""))
if op == "count":
    print(len(rows))
    sys.exit(0)
vals = []
for n, v in rows:
    if not (v.isascii() and v.isdigit()):
        fail("error: bad value %s on line %d" % (v, n), 3)
    vals.append(int(v))
if op == "sum":
    print(sum(vals))
elif not vals:
    print("n/a")
elif op == "min":
    print(min(vals))
elif op == "max":
    print(max(vals))
else:
    print(sum(vals) // len(vals))
