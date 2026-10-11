import sys


def parse(line):
    fields = []
    i = 0
    n = len(line)
    while True:
        if i < n and line[i] == '"':
            i += 1
            buf = []
            while i < n:
                if line[i] == '"':
                    if i + 1 < n and line[i + 1] == '"':
                        buf.append('"')
                        i += 2
                        continue
                    i += 1
                    break
                buf.append(line[i])
                i += 1
            fields.append("".join(buf))
        else:
            j = line.find(",", i)
            if j < 0:
                j = n
            fields.append(line[i:j])
            i = j
        if i < n and line[i] == ",":
            i += 1
            continue
        break
    return fields


lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
for line in lines:
    f = parse(line)
    print("%d: %s" % (len(f), "|".join("[" + x + "]" for x in f)))
