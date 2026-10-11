import sys

args = sys.argv[1:]
if len(args) != 3:
    print("usage: transform INPUT OUTPUT MODE")
    sys.exit(2)
inp, outp, mode = args
try:
    with open(inp, "rb") as f:
        text = f.read().decode("latin-1")
except OSError:
    print("error: cannot read " + inp)
    sys.exit(1)
lines = text.split("\n")
if lines and lines[-1] == "":
    lines.pop()
if mode == "upper":
    res = [l.upper() for l in lines]
elif mode == "reverse":
    res = lines[::-1]
elif mode == "sort":
    res = sorted(lines)
elif mode == "dedupe":
    seen = set()
    res = []
    for l in lines:
        if l not in seen:
            seen.add(l)
            res.append(l)
elif mode == "trim":
    res = [l.strip(" ") for l in lines]
elif mode == "number":
    res = ["%d:%s" % (i, l) for i, l in enumerate(lines, 1)]
else:
    print("error: unknown mode " + mode)
    sys.exit(2)
with open(outp, "wb") as f:
    f.write("".join(l + "\n" for l in res).encode("latin-1"))
with open(outp, "rb") as f:
    back = f.read()
sys.stdout.buffer.write(("wrote %d lines to %s\n" % (len(res), outp)).encode() + back)
