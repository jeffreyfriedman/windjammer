import sys
import re

text = sys.stdin.read()
lines = text.split("\n")
if lines and lines[-1] == "":
    lines.pop()
for line in lines:
    mode = line[:1]
    body = line[2:]
    if mode == "E":
        out = []
        i = 0
        while i < len(body):
            j = i
            while j < len(body) and body[j] == body[i]:
                j += 1
            out.append("%d%s" % (j - i, body[i]))
            i = j
        print("".join(out))
    else:
        print("".join(ch * int(n) for n, ch in re.findall(r"([0-9]+)([a-z])", body)))
