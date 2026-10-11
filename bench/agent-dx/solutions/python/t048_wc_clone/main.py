import sys


def count(data):
    return data.count(b"\n"), len(data.split()), len(data)


args = sys.argv[1:]
if not args:
    l, w, b = count(sys.stdin.buffer.read())
    print(l, w, b)
    sys.exit(0)
tl = tw = tb = 0
failed = False
for name in args:
    try:
        with open(name, "rb") as f:
            data = f.read()
    except OSError:
        print("wc: %s: No such file" % name)
        failed = True
        continue
    l, w, b = count(data)
    tl, tw, tb = tl + l, tw + w, tb + b
    print(l, w, b, name)
if len(args) > 1:
    print(tl, tw, tb, "total")
sys.exit(1 if failed else 0)
