def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["stock bolt 100 5", "stock nut 50 2", "sell bolt 30", "report"]), _c(["report", "sell x 1", "remove x"])]
EDGE = [_c(["stock a 1 10", "stock a 1 11", "stock a 0 10", "sell a 5", "sell a 0", "remove a", "sell a 1", "remove a", "report", "stock b 3 7", "report"])]


def rand(rng):
    cmds = []
    for _ in range(rng.randint(5, 30)):
        s = rng.choice(["ax", "by", "cz"])
        r = rng.random()
        cmds.append(f"stock {s} {rng.randint(0, 20)} {rng.choice([3, 3, 5])}" if r < 0.35 else f"sell {s} {rng.randint(0, 25)}" if r < 0.7 else f"remove {s}" if r < 0.8 else "report")
    return _c(cmds)


def solve(case):
    inv, out = {}, []
    for ln in case["stdin"].split("\n")[:-1]:
        p = ln.split()
        c = p[0]
        if c == "stock":
            s, q, pr = p[1], int(p[2]), int(p[3])
            if q <= 0:
                out.append("error: bad quantity")
            elif s in inv and inv[s][1] != pr:
                out.append(f"error: price mismatch for {s}")
            else:
                if s not in inv:
                    inv[s] = [0, pr]
                inv[s][0] += q
                out.append(f"ok {inv[s][0]}")
        elif c == "sell":
            s, q = p[1], int(p[2])
            if q <= 0:
                out.append("error: bad quantity")
            elif s not in inv:
                out.append(f"error: unknown sku {s}")
            elif inv[s][0] < q:
                out.append(f"error: insufficient stock for {s} (have {inv[s][0]}, need {q})")
            else:
                inv[s][0] -= q
                out.append(f"sold {q} for {q * inv[s][1]}")
        elif c == "remove":
            s = p[1]
            if s not in inv:
                out.append(f"error: unknown sku {s}")
            elif inv[s][0] > 0:
                out.append(f"error: not empty {s}")
            else:
                del inv[s]
                out.append(f"removed {s}")
        else:
            if not inv:
                out.append("(empty)")
            else:
                for s in sorted(inv):
                    out.append(f"{s} {inv[s][0]} x {inv[s][1]} = {inv[s][0] * inv[s][1]}")
                out.append(f"total value {sum(q * pr for q, pr in inv.values())}")
    return "\n".join(out) + ("\n" if out else "")
