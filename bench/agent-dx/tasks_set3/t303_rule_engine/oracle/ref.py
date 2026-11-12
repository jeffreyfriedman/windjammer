def _c(stdin): return {"stdin": stdin}
SAMPLES = [
    _c("item apple 6 100\nitem pear 2 250\nrule percent apple 10\nrule bogo pear 1 1\nrule threshold 500 10\n"),
    _c("item a 3 333\nrule fixed * 50\nrule percent * 33\nbogus\nrule percent a 101\nitem b x 4\n"),
]
EDGE = [
    _c(""),
    _c("rule percent * 10\nrule threshold 0 50\n"),
    _c("item a 1 0\nrule bogo a 1 1\nrule percent a 100\nrule fixed a 5\n"),
    _c("item a 7 100\nitem a 7 100\nrule bogo a 2 1\nrule bogo * 3 1\n"),
    _c("item a 2 100\nrule threshold 200 100\nrule fixed a 50\nrule threshold 0 50\n"),
    _c("\n\nitem a 1 1 1\nitem a\nitem a 0 5\nitem a 1 -5\nrule\nrule fixed a\nrule bogo a 0 1\nrule bogo a 1 0\nrule threshold 5\nrule percent a 100 \nrule discount a 5\nrule fixed * 1234567890\nitem  a 1 5\n"),
    _c("item x 10 999\nrule percent x 7\nrule percent x 7\nrule percent * 7\nrule fixed x 1\nrule threshold 9000 3\nrule threshold 8000 3\n"),
    _c("item a 5 10\nitem b 5 10\nrule threshold 100 20\nrule percent a 50\nrule fixed b 100\n"),
    _c("item a 1 10\nrule percent b 10\nrule fixed c 5\nrule bogo d 1 1\nrule threshold 11 10\n"),
]
SKUS = ["apple", "pear", "fig", "a1"]
def rand(rng):
    lines = []
    for _ in range(rng.randint(1, 6)):
        lines.append(f"item {rng.choice(SKUS)} {rng.randint(1, 12)} {rng.choice([0, 1, 50, 99, 100, 250, 1999])}")
    for _ in range(rng.randint(1, 7)):
        t = rng.choice(SKUS + ["*", "*"])
        k = rng.choice(["percent", "fixed", "bogo", "threshold"])
        if k == "percent": lines.append(f"rule percent {t} {rng.randint(0, 100)}")
        elif k == "fixed": lines.append(f"rule fixed {t} {rng.choice([0, 10, 75, 300])}")
        elif k == "bogo": lines.append(f"rule bogo {t} {rng.randint(1, 4)} {rng.randint(1, 3)}")
        else: lines.append(f"rule threshold {rng.choice([0, 100, 1000, 3000])} {rng.randint(0, 100)}")
    if rng.random() < 0.3:
        lines.insert(rng.randint(0, len(lines)), rng.choice(["junk", "item a b c", "rule percent a 200", "", "rule bogo a 1", "item fig 0 5"]))
    rng.shuffle(lines) if rng.random() < 0.3 else None
    return _c("\n".join(lines) + "\n")
def _num(s, lo=0, hi=10**9 - 1):
    return s.isascii() and s.isdigit() and len(s) <= 9 and lo <= int(s) <= hi
def solve(case):
    errs, items, rules = [], [], []
    for n, ln in enumerate(case["stdin"].split("\n"), 1):
        if ln == "": continue
        p = ln.split(" ")
        ok = False
        if p[0] == "item" and len(p) == 4 and p[1] != "" and _num(p[2], 1) and _num(p[3]):
            items.append({"sku": p[1], "qty": int(p[2]), "unit": int(p[3])}); ok = True
        elif p[0] == "rule" and len(p) >= 2:
            k = p[1]
            if k == "percent" and len(p) == 4 and p[2] != "" and _num(p[3], 0, 100):
                rules.append((k, p[2], int(p[3]))); ok = True
            elif k == "fixed" and len(p) == 4 and p[2] != "" and _num(p[3]):
                rules.append((k, p[2], int(p[3]))); ok = True
            elif k == "bogo" and len(p) == 5 and p[2] != "" and _num(p[3], 1) and _num(p[4], 1):
                rules.append((k, p[2], int(p[3]), int(p[4]))); ok = True
            elif k == "threshold" and len(p) == 4 and _num(p[2]) and _num(p[3], 0, 100):
                rules.append((k, int(p[2]), int(p[3]))); ok = True
        if not ok: errs.append(f"error line {n}: {ln}")
    if case["stdin"].endswith("\n") and errs and False: pass
    for it in items: it["cur"] = it["qty"] * it["unit"]; it["base"] = it["cur"]
    cart, out = 0, []
    for i, r in enumerate(rules, 1):
        saved = 0
        if r[0] == "threshold":
            running = sum(it["cur"] for it in items) - cart
            if running >= r[1]:
                saved = running * r[2] // 100; cart += saved
        else:
            for it in items:
                if r[1] != "*" and r[1] != it["sku"]: continue
                if r[0] == "percent": d = it["cur"] * r[2] // 100
                elif r[0] == "fixed": d = min(it["cur"], r[2])
                else: d = min(it["cur"], (it["qty"] // (r[2] + r[3])) * r[3] * it["unit"])
                it["cur"] -= d; saved += d
        out.append(f"rule {i} {r[0]} {saved}")
    res = list(errs)
    res += [f"item {it['sku']} {it['base']} {it['cur']}" for it in items]
    res += out
    res.append(f"total {max(0, sum(it['cur'] for it in items) - cart)}")
    return "".join(x + "\n" for x in res), (1 if errs else 0)
