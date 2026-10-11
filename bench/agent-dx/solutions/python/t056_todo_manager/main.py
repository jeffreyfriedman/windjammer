import sys

items = {}  # id -> [text, done]
next_id = 1
for line in sys.stdin.read().split("\n"):
    line = line.strip()
    if not line:
        continue
    parts = line.split(None, 1)
    cmd = parts[0]
    arg = parts[1] if len(parts) > 1 else None
    if cmd == "add":
        if arg is None:
            print("error: missing argument")
        else:
            items[next_id] = [arg, False]
            print("added %d" % next_id)
            next_id += 1
    elif cmd in ("done", "del"):
        if arg is None:
            print("error: missing argument")
            continue
        try:
            k = int(arg.split()[0])
        except ValueError:
            k = None
        if k not in items:
            print("error: no such item " + arg.split()[0])
        elif cmd == "done":
            if items[k][1]:
                print("error: already done %d" % k)
            else:
                items[k][1] = True
                print("done %d" % k)
        else:
            del items[k]
            print("deleted %d" % k)
    elif cmd == "list":
        f = arg.split()[0] if arg else "all"
        if f not in ("all", "open", "done"):
            print("error: bad filter " + f)
            continue
        shown = [(k, v) for k, v in sorted(items.items())
                 if f == "all" or (f == "done") == v[1]]
        if not shown:
            print("(none)")
        for k, v in shown:
            print("%d [%s] %s" % (k, "x" if v[1] else " ", v[0]))
    elif cmd == "clear":
        gone = [k for k, v in items.items() if v[1]]
        for k in gone:
            del items[k]
        print("cleared %d" % len(gone))
    elif cmd == "count":
        d = sum(1 for v in items.values() if v[1])
        print("open=%d done=%d" % (len(items) - d, d))
    else:
        print("error: unknown command " + cmd)
