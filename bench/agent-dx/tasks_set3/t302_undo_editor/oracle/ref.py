def _c(stdin): return {"stdin": stdin}
SAMPLES = [
    _c("insert 0 hello world\ninsert 1 bye\nswap 0 1\nprint\nundo\nprint\ndelete 5\n"),
    _c("insert 0 a\ninsert 1 b\nreplace 0 z\nundo\nundo\nredo\nprint\ninsert 0 q\nredo\nprint\n"),
]
EDGE = [
    _c(""),
    _c("print\nundo\nredo\n"),
    _c("insert 0 a\ninsert 0 b\ninsert 0 c\ninsert 0 d\ninsert 0 e\ninsert 0 f\nundo\nundo\nundo\nundo\nundo\nundo\nprint\nredo\nredo\nredo\nredo\nredo\nredo\nprint\n"),
    _c("insert x a\ninsert 1 a\ninsert 0\ninsert 0 \ninsert 0 a b  c\nprint\nfoo\ndelete\ndelete 0 1\nswap 0\nswap 0 0 0\nundo 1\nprint now\ninsert 1234567 a\ninsert -1 a\n"),
    _c("insert 0 a\ninsert 1 b\nswap 0 0\nundo\nprint\nswap 0 2\nswap 2 0\nreplace 2 x\ndelete 2\nundo\nundo\nundo\nundo\nprint\n"),
    _c("insert 0 one\ninsert 0 two\ndelete 1\nundo\nredo\nredo\nprint\nreplace 0 TWO\nundo\nprint\nredo\nprint\n"),
    _c("insert 0 a\nundo\nredo\ninsert 0 b\nredo\nundo\nundo\nundo\nprint\n"),
    _c("\ninsert 0 x\n\nUNDO\nundo\nundo\n"),
    _c("insert 0 a\ninsert 1 b\ninsert 2 c\ndelete 0\ndelete 0\ndelete 0\nprint\nundo\nundo\nprint\nundo\nundo\nprint\nredo\nredo\nredo\nredo\nprint\n"),
]
def rand(rng):
    lines = []
    words = ["alpha", "beta x", "g", "delta  d", "e f g"]
    for _ in range(rng.randint(6, 30)):
        r = rng.random()
        n = rng.randint(0, 5)
        if r < 0.25: lines.append(f"insert {rng.randint(0, 5)} {rng.choice(words)}")
        elif r < 0.35: lines.append(f"delete {n}")
        elif r < 0.45: lines.append(f"replace {n} {rng.choice(words)}")
        elif r < 0.55: lines.append(f"swap {n} {rng.randint(0, 5)}")
        elif r < 0.7: lines.append("undo")
        elif r < 0.8: lines.append("redo")
        elif r < 0.93: lines.append("print")
        else: lines.append(rng.choice(["bogus", "insert z q", "delete", "undo 2", "replace 1", "", "insert 1 "]))
    lines.append("print")
    return _c("\n".join(lines) + "\n")
def solve(case):
    buf, undo, redo, out, errs = [], [], [], [], 0
    def err(m):
        nonlocal errs
        errs += 1; out.append("error: " + m)
    def num(s): return s != "" and s.isascii() and s.isdigit() and len(s) <= 6
    def apply(e):
        k = e[0]
        if k == "ins": buf.insert(e[1], e[2])
        elif k == "del": buf.pop(e[1])
        elif k == "rep": buf[e[1]] = e[3]
        elif k == "swp": buf[e[1]], buf[e[2]] = buf[e[2]], buf[e[1]]
    def revert(e):
        k = e[0]
        if k == "ins": buf.pop(e[1])
        elif k == "del": buf.insert(e[1], e[2])
        elif k == "rep": buf[e[1]] = e[2]
        elif k == "swp": buf[e[1]], buf[e[2]] = buf[e[2]], buf[e[1]]
    for ln in case["stdin"].split("\n"):
        if ln == "": continue
        w = ln.split(" ")
        c = w[0]
        if c in ("insert", "replace"):
            p = ln.split(" ", 2)
            if len(p) != 3 or not num(p[1]) or p[2] == "": err("bad arguments"); continue
            n = int(p[1])
            if c == "insert":
                if n > len(buf): err("bad index"); continue
                e = ("ins", n, p[2])
            else:
                if n >= len(buf): err("bad index"); continue
                e = ("rep", n, buf[n], p[2])
        elif c == "delete":
            if len(w) != 2 or not num(w[1]): err("bad arguments"); continue
            n = int(w[1])
            if n >= len(buf): err("bad index"); continue
            e = ("del", n, buf[n])
        elif c == "swap":
            if len(w) != 3 or not num(w[1]) or not num(w[2]): err("bad arguments"); continue
            a, b = int(w[1]), int(w[2])
            if a >= len(buf) or b >= len(buf): err("bad index"); continue
            e = ("swp", a, b)
        elif c in ("undo", "redo", "print"):
            if len(w) != 1: err("bad arguments"); continue
            if c == "print":
                if not buf: out.append("(empty)")
                for i, t in enumerate(buf): out.append(f"{i}: {t}")
            elif c == "undo":
                if not undo: err("nothing to undo"); continue
                e = undo.pop(); revert(e); redo.append(e)
            else:
                if not redo: err("nothing to redo"); continue
                e = redo.pop(); apply(e); undo.append(e)
            continue
        else:
            err("unknown command"); continue
        apply(e); undo.append(e); redo.clear()
        if len(undo) > 5: undo.pop(0)
    return "".join(x + "\n" for x in out), (1 if errs else 0)
