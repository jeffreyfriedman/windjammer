def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["coin 100", "coin 100", "select cola", "status"]), _c(["coin 25", "select gum", "coin 10", "select gum", "refund", "coin 3"])]
EDGE = [_c(["status", "refund", "select pie", "coin 100", "select chips", "coin 100", "select chips", "coin 100", "select chips", "coin 100", "coin 100", "select chips", "status"]),
        _c(["coin 100", "coin 100", "coin 100", "coin 25", "select cola", "coin 100", "coin 100", "select cola", "coin 100", "coin 100", "select cola", "coin 100", "coin 100", "select cola", "status", "coin 5", "refund"])]


def rand(rng):
    cmds = []
    for _ in range(rng.randint(8, 50)):
        r = rng.random()
        cmds.append(f"coin {rng.choice([5, 10, 25, 100, 100, 2, 50])}" if r < 0.5 else f"select {rng.choice(['cola', 'chips', 'gum', 'gum', 'tea'])}" if r < 0.8 else rng.choice(["refund", "status"]))
    return _c(cmds)


def change(c):
    o = []
    for d in (100, 25, 10, 5):
        while c >= d:
            o.append(d)
            c -= d
    return o


def solve(case):
    price = {"cola": 150, "chips": 100, "gum": 35}
    stock = {"cola": 3, "chips": 2, "gum": 5}
    cr, out = 0, []
    for ln in case["stdin"].split("\n")[:-1]:
        p = ln.split()
        if p[0] == "coin":
            v = int(p[1])
            if v in (5, 10, 25, 100):
                cr += v
                out.append(f"credit {cr}")
            else:
                out.append(f"rejected {v}")
        elif p[0] == "select":
            it = p[1]
            if it not in price:
                out.append(f"error: unknown item {it}")
            elif stock[it] == 0:
                out.append(f"error: sold out {it}")
            elif cr < price[it]:
                out.append(f"error: need {price[it] - cr} more")
            else:
                stock[it] -= 1
                ch = change(cr - price[it])
                cr = 0
                out.append(f"dispensed {it} change: " + (" ".join(map(str, ch)) if ch else "none"))
        elif p[0] == "refund":
            ch = change(cr)
            cr = 0
            out.append("refund: " + (" ".join(map(str, ch)) if ch else "none"))
        else:
            out.append(f"credit={cr} cola={stock['cola']} chips={stock['chips']} gum={stock['gum']}")
    return "\n".join(out) + ("\n" if out else "")
