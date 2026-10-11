def _c(cmds):
    return {"stdin": "".join(c + "\n" for c in cmds)}


SAMPLES = [_c(["open a 100", "open b 0", "transfer a b 30", "balance a", "withdraw b 50", "deposit b 5"]), _c(["deposit z 5", "open a 1", "open a 2"])]
EDGE = [_c(["open a 5", "transfer a a 1", "transfer a a 100", "transfer a q 1", "transfer q a 1", "deposit a 0", "withdraw a -3", "open b -1", "balance q"])]


def rand(rng):
    names = ["al", "bo", "cy"]
    cmds = []
    for _ in range(rng.randint(5, 40)):
        r = rng.random()
        n, m, a = rng.choice(names), rng.choice(names), rng.randint(-2, 120)
        cmds.append(f"open {n} {a}" if r < 0.2 else f"deposit {n} {a}" if r < 0.4 else f"withdraw {n} {a}" if r < 0.6
                    else f"transfer {n} {m} {a}" if r < 0.85 else f"balance {n}")
    return _c(cmds)


def solve(case):
    acc, out = {}, []
    for ln in case["stdin"].split("\n")[:-1]:
        p = ln.split()
        c = p[0]
        if c == "open":
            a = int(p[2])
            if a < 0:
                out.append("error: invalid amount")
            elif p[1] in acc:
                out.append("error: duplicate account")
            else:
                acc[p[1]] = a
                out.append("ok")
        elif c == "balance":
            out.append(str(acc[p[1]]) if p[1] in acc else f"error: unknown account {p[1]}")
        elif c in ("deposit", "withdraw"):
            a = int(p[2])
            if a <= 0:
                out.append("error: invalid amount")
            elif p[1] not in acc:
                out.append(f"error: unknown account {p[1]}")
            elif c == "withdraw" and acc[p[1]] < a:
                out.append("error: insufficient funds")
            else:
                acc[p[1]] += a if c == "deposit" else -a
                out.append("ok")
        else:
            f, t, a = p[1], p[2], int(p[3])
            if a <= 0:
                out.append("error: invalid amount")
            elif f not in acc:
                out.append(f"error: unknown account {f}")
            elif t not in acc:
                out.append(f"error: unknown account {t}")
            elif f == t:
                out.append("error: same account")
            elif acc[f] < a:
                out.append("error: insufficient funds")
            else:
                acc[f] -= a
                acc[t] += a
                out.append("ok")
    out += [f"{k} {acc[k]}" for k in sorted(acc)]
    return "\n".join(out) + "\n"
