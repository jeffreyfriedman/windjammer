def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["add buy milk", "add write code", "done 1", "list", "list open", "count"]), _c(["list", "done 3", "bogus"])]
EDGE = [_c(["add a", "add b", "del 1", "add c", "list", "done 2", "done 2", "clear", "list done", "list x", "done", "del", "", "add", "count", "add two  spaces"])]


def rand(rng):
    cmds = []
    for _ in range(rng.randint(5, 40)):
        r = rng.random()
        i = rng.randint(1, 6)
        cmds.append(f"add item {rng.randint(0, 99)}" if r < 0.3 else f"done {i}" if r < 0.45 else f"del {i}" if r < 0.55 else
                    rng.choice(["list", "list open", "list done", "list all", "clear", "count", "list zz", "nope 1", "done", ""]))
    return _c(cmds)


def solve(case):
    items, nid, out = {}, 1, []
    for ln in case["stdin"].split("\n")[:-1]:
        if not ln.strip():
            continue
        cmd, _, rest = ln.partition(" ")
        if cmd == "add":
            if rest == "":
                out.append("error: missing argument")
                continue
            items[nid] = [rest, False]
            out.append(f"added {nid}")
            nid += 1
        elif cmd in ("done", "del"):
            if rest == "":
                out.append("error: missing argument")
                continue
            i = int(rest)
            if i not in items:
                out.append(f"error: no such item {i}")
            elif cmd == "del":
                del items[i]
                out.append(f"deleted {i}")
            elif items[i][1]:
                out.append(f"error: already done {i}")
            else:
                items[i][1] = True
                out.append(f"done {i}")
        elif cmd == "list":
            f = rest or "all"
            if f not in ("all", "open", "done"):
                out.append(f"error: bad filter {f}")
                continue
            sel = [(i, t) for i, t in sorted(items.items()) if f == "all" or (f == "done") == t[1]]
            if not sel:
                out.append("(none)")
            for i, (txt, d) in sel:
                out.append(f"{i} [{'x' if d else ' '}] {txt}")
        elif cmd == "clear":
            ds = [i for i, t in items.items() if t[1]]
            for i in ds:
                del items[i]
            out.append(f"cleared {len(ds)}")
        elif cmd == "count":
            out.append(f"open={sum(1 for t in items.values() if not t[1])} done={sum(1 for t in items.values() if t[1])}")
        else:
            out.append(f"error: unknown command {cmd}")
    return "\n".join(out) + ("\n" if out else "")
