import sys

flags = set()
rest = []
for a in sys.argv[1:]:
    if a in ("-i", "-n", "-c", "-v"):
        flags.add(a)
    else:
        rest.append(a)
pattern, files = rest[0], rest[1:]
multi = len(files) > 1
out = []
total = 0
missing = False
pat = pattern.lower() if "-i" in flags else pattern
for name in files:
    try:
        with open(name, "rb") as f:
            text = f.read().decode("latin-1")
    except OSError:
        out.append("grep: %s: No such file" % name)
        missing = True
        continue
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    cnt = 0
    for i, line in enumerate(lines, 1):
        hay = line.lower() if "-i" in flags else line
        if (pat in hay) != ("-v" in flags):
            cnt += 1
            if "-c" not in flags:
                pre = (name + ":" if multi else "") + (str(i) + ":" if "-n" in flags else "")
                out.append(pre + line)
    total += cnt
    if "-c" in flags:
        out.append((name + ":" if multi else "") + str(cnt))
sys.stdout.buffer.write("".join(l + "\n" for l in out).encode("latin-1", "replace"))
sys.exit(2 if missing else (0 if total > 0 else 1))
