def _c(stdin): return {"stdin": stdin}
EX = "open ann 100\nopen bob 20\nopen cy 0\niou ann bob 30\niou bob cy 50\niou cy ann 10\nsettle\nall\nhistory bob\ntransfer cy ann 50\n"
SAMPLES = [
    _c(EX),
    _c("open a 10\nopen b 5\ntransfer a b 7\ntransfer a b 7\ntransfer b a 12\nbalance a\nbalance b\nbalance zed\nopen a 1\ntransfer a a 1\n"),
]
EDGE = [
    _c(""),
    _c("all\nsettle\nbalance x\nhistory x\n"),
    _c("open a 0\nopen b 0\nopen c 0\niou a b 5\niou b c 5\nsettle\nsettle\nall\nhistory a\nhistory c\n"),
    _c("open a 10\nopen b 10\niou a b 5\niou b a 5\nsettle\nhistory a\n"),
    _c("open a 100\nopen b 0\nopen c 0\nopen d 0\niou a b 10\niou a c 10\niou d a 5\nsettle\nall\n"),
    _c("open x 50\nopen y 50\nopen z 50\nopen w 50\niou x w 30\niou y w 30\niou z w 20\niou x z 1\nsettle\nall\n"),
    _c("open a 1\nopen b 1\niou a b 100\nsettle\nhistory a\nhistory b\ntransfer a b 1\ntransfer a b 1\nhistory a\n"),
    _c("open a\nopen a b c\nopen a -1\nopen a 1x\nopen a 1234567890\ntransfer a b\ntransfer a b 0\niou a b 0\nfoo\nbalance\nall now\nsettle now\nhistory\n open a 1\n"),
    _c("open a 5\niou q a 3\niou a q 3\niou a a 3\ntransfer q a 1\ntransfer a q 1\ntransfer a a 1\nopen b 5\ntransfer b a 6\nhistory b\n"),
    _c("open m 10\nopen n 10\nopen o 10\niou m n 7\niou m o 7\niou n o 7\niou o m 7\nsettle\nall\nhistory m\n"),
]
NAMES = ["ann", "bob", "cy", "dee", "Eve", "f1"]
def rand(rng):
    names = rng.sample(NAMES, rng.randint(2, 5))
    lines = [f"open {n} {rng.choice([0, 10, 50, 100, 500])}" for n in names]
    for _ in range(rng.randint(5, 22)):
        r = rng.random()
        a, b = rng.choice(names), rng.choice(names)
        if r < 0.3: lines.append(f"transfer {a} {b} {rng.randint(1, 120)}")
        elif r < 0.65: lines.append(f"iou {a} {b} {rng.randint(1, 80)}")
        elif r < 0.78: lines.append("settle")
        elif r < 0.85: lines.append(f"balance {rng.choice(names + ['nobody'])}")
        elif r < 0.9: lines.append(f"history {a}")
        elif r < 0.95: lines.append("all")
        else: lines.append(rng.choice(["bogus", "transfer a", "iou x y z w", "open dup"]))
    lines.append("settle"); lines.append("all")
    return _c("\n".join(lines) + "\n")
def _amt(s, lo):
    return s.isascii() and s.isdigit() and len(s) <= 9 and int(s) >= lo
def solve(case):
    bal, hist, book, out, errs = {}, {}, [], [], 0
    def err(m):
        nonlocal errs
        errs += 1; out.append("error: " + m)
    def move(a, b, n):
        bal[a] -= n; bal[b] += n
        hist[a].append(f"out {n} to {b}"); hist[b].append(f"in {n} from {a}")
    for ln in case["stdin"].split("\n"):
        if ln == "": continue
        p = ln.split(" ")
        c = p[0]
        if "" in p: err("bad command"); continue
        if c == "open" and len(p) == 3 and _amt(p[2], 0):
            if p[1] in bal: err(f"account exists {p[1]}")
            else: bal[p[1]] = int(p[2]); hist[p[1]] = [f"open {int(p[2])}"]
        elif c in ("transfer", "iou") and len(p) == 4 and _amt(p[3], 1):
            a, b, n = p[1], p[2], int(p[3])
            if a not in bal: err(f"unknown account {a}")
            elif b not in bal: err(f"unknown account {b}")
            elif a == b: err("self transfer")
            elif c == "iou": book.append((a, b, n))
            elif bal[a] < n: err(f"overdraft {a}")
            else: move(a, b, n)
        elif c == "balance" and len(p) == 2:
            if p[1] in bal: out.append(f"{p[1]}: {bal[p[1]]}")
            else: err(f"unknown account {p[1]}")
        elif c == "all" and len(p) == 1:
            if not bal: out.append("(no accounts)")
            for n in sorted(bal): out.append(f"{n}: {bal[n]}")
        elif c == "history" and len(p) == 2:
            if p[1] in bal: out.extend(hist[p[1]])
            else: err(f"unknown account {p[1]}")
        elif c == "settle" and len(p) == 1:
            net = {}
            for a, b, n in book:
                net[a] = net.get(a, 0) - n; net[b] = net.get(b, 0) + n
            book = []
            steps = 0
            while True:
                deb = sorted((n for n in net if net[n] < 0), key=lambda n: (net[n], n))
                cre = sorted((n for n in net if net[n] > 0), key=lambda n: (-net[n], n))
                if not deb: break
                d, cr = deb[0], cre[0]
                amt = min(-net[d], net[cr])
                line = f"{d} pays {cr} {amt}"
                if bal[d] >= amt: move(d, cr, amt)
                else: line += " (failed: overdraft)"
                net[d] += amt; net[cr] -= amt
                out.append(line); steps += 1
            if not steps: out.append("nothing to settle")
        else:
            err("bad command")
    return "".join(x + "\n" for x in out), (1 if errs else 0)
