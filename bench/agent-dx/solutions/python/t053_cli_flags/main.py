import sys

args = sys.argv[1:]
name = "anonymous"
count = 1
verbosity = 0
dry = False
pos = []


def err(msg):
    print(msg)
    sys.exit(2)


def parse_count(v):
    if not (v.isascii() and v.isdigit()):
        err("error: invalid count " + v)
    return int(v)


i = 0
while i < len(args):
    a = args[i]
    i += 1
    if a == "--":
        pos.extend(args[i:])
        break
    if a == "--name" or a == "--count":
        if i >= len(args):
            err("error: missing value for " + a)
        v = args[i]
        i += 1
        if a == "--name":
            name = v
        else:
            count = parse_count(v)
    elif a.startswith("--name="):
        name = a[len("--name="):]
    elif a.startswith("--count="):
        count = parse_count(a[len("--count="):])
    elif a == "--dry-run":
        dry = True
    elif len(a) >= 2 and a[0] == "-" and a[1] != "-" and set(a[1:]) == {"v"}:
        verbosity += len(a) - 1
    elif a.startswith("-") and a != "-":
        err("error: unknown option " + a)
    else:
        pos.append(a)
print("name: " + name)
print("count: %d" % count)
print("verbosity: %d" % verbosity)
print("dry-run: " + ("true" if dry else "false"))
print("positional: " + " ".join(pos))
