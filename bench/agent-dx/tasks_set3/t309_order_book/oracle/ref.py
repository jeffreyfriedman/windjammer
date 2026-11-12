def _c(stdin): return {"stdin": stdin}
SAMPLES = [
    _c("sell ann 10 101\nsell bob 5 100\nbuy cy 12 101\nbook\npos\ncancel ann 1\nbuy cy 3 99\nbook\n"),
    _c("buy ann 5 50\nbuy ann 5 50 ioc\nsell ann 8 49\nsell bob 8 49 fok\nsell bob 4 50\namend ann 1 2\namend bob 9 1\nbook\n"),
]
EDGE = [
    _c(""),
    _c("book\npos\n"),
    _c("buy a 10 100\nbuy b 10 100\nbuy c 10 100\nsell d 25 100\nbook\npos\n"),
    _c("buy a 10 100\nbuy b 10 100\namend a 1 5\nsell c 5 100\nbook\namend a 1 20\nsell c 8 100\nbook\n"),
    _c("buy a 10 100\nbuy b 10 100\namend a 1 10\nsell c 3 100\nbuy a 5 100\nsell c 12 100\nbook\n"),
    _c("buy a 5 100\nbuy a 5 101\nbuy b 5 99\nsell a 12 98\nbook\npos\n"),
    _c("sell a 5 100\nsell b 5 100\nbuy a 10 100 fok\nbook\npos\n"),
    _c("sell a 5 100\nsell b 5 101\nbuy c 10 100 fok\nbuy c 10 101 fok\nbook\npos\n"),
    _c("sell a 5 100\nbuy c 3 100 ioc\nbuy c 9 100 ioc\nbuy c 1 100 ioc\nbook\n"),
    _c("sell a 5 100\nsell b 5 102\nmbuy c 7\nmbuy c 100\nmsell d 4\nbook\npos\n"),
    _c("buy a 5 100\nbuy b 5 100\ncancel a 1\ncancel a 1\ncancel b 1\ncancel b 2\ncancel b 2\ncancel b 99\nsell c 5 100\nbook\n"),
    _c("buy a 0 100\nbuy a 5 0\nbuy a 5 100001\nbuy a 1000001 5\nbuy a 5 100 gtc\nbuy a x 100\nbuy 5 100\nfoo\nsell a 1 1\nbuy b 1 1 fok\nbuy b 1 1\ncancel a x\ncancel a\namend a 2 0\namend a 2 -1\namend\n  book\nbook now\n"),
    _c("buy a 10 100\nsell b 4 100\nsell b 4 100\namend a 1 2\namend a 1 2\namend a 1 3\nsell c 1 100\nbook\npos\n"),
    _c("buy a 1000000 100000\nsell b 1000000 1\nsell c 1 1\nbuy d 1 100000\nbook\npos\n"),
]
def rand(rng):
    ts = ["ann", "bob", "cy", "dee"][:rng.randint(2, 4)]
    lines, owner = [], {}
    for _ in range(rng.randint(30, 60)):
        r = rng.random()
        t = rng.choice(ts)
        px = rng.randint(95, 105)
        q = rng.randint(1, 12)
        tif = rng.choice(["", "", "", " ioc", " fok"])
        if owner and r >= 0.68 and r < 0.88:
            oid = rng.choice(list(owner))
            who = owner[oid] if rng.random() < 0.85 else rng.choice(ts)
            if r < 0.78: lines.append(f"cancel {who} {oid}")
            else: lines.append(f"amend {who} {oid} {rng.randint(1, 15)}")
            continue
        if r < 0.3 or (0.68 <= r < 0.88 and not owner):
            lines.append(f"buy {t} {q} {px}{tif}"); owner[len(owner) + 1] = t
        elif r < 0.6:
            lines.append(f"sell {t} {q} {px}{tif}"); owner[len(owner) + 1] = t
        elif r < 0.64:
            lines.append(f"mbuy {t} {q}"); owner[len(owner) + 1] = t
        elif r < 0.68:
            lines.append(f"msell {t} {q}"); owner[len(owner) + 1] = t
        elif r < 0.95: lines.append("book")
        elif r < 0.98: lines.append("pos")
        else: lines.append(rng.choice(["buy x", "oops", "buy a 1 1 zzz", "cancel"]))
    lines.append("book"); lines.append("pos")
    return _c("\n".join(lines) + "\n")

def solve(case):
    bids, asks = {}, {}      # price -> list of order dicts in priority order
    orders = {}              # resting id -> (order dict, side)
    pos = {}
    out, errs, nid = [], 0, [0]
    def err(m):
        nonlocal errs
        errs += 1; out.append("error: " + m)
    def num(s, lo, hi): return s.isascii() and s.isdigit() and len(s) <= 7 and lo <= int(s) <= hi
    def trader(t):
        pos.setdefault(t, [0, 0])
    def best(opp, side):
        # best price level on the opposite side
        if not opp: return None
        return min(opp) if side == "buy" else max(opp)
    def submit(side, t, q, limit, tif):
        trader(t)
        nid[0] += 1; oid = nid[0]
        out.append(f"order {oid}")
        opp = asks if side == "buy" else bids
        def crosses(px): return limit is None or (px <= limit if side == "buy" else px >= limit)
        if tif == "fok":
            avail = sum(o["q"] for px, lvl in opp.items() if crosses(px) for o in lvl if o["t"] != t)
            if avail < q:
                out.append(f"reject {oid} fok"); return
        rem = q
        while rem > 0:
            px = best(opp, side)
            if px is None or not crosses(px): break
            lvl = opp[px]; r = lvl[0]
            if r["t"] == t:
                lvl.pop(0); del orders[r["id"]]
                if not lvl: del opp[px]
                out.append(f"cancel {r['id']} self-trade"); continue
            f = min(rem, r["q"])
            bid, sid = (oid, r["id"]) if side == "buy" else (r["id"], oid)
            out.append(f"trade {bid} {sid} {f} @ {px}")
            buyer, seller = (t, r["t"]) if side == "buy" else (r["t"], t)
            pos[buyer][0] += f; pos[buyer][1] -= f * px
            pos[seller][0] -= f; pos[seller][1] += f * px
            rem -= f; r["q"] -= f
            if r["q"] == 0:
                lvl.pop(0); del orders[r["id"]]
                if not lvl: del opp[px]
        if rem > 0:
            if tif in ("ioc", "mkt"): out.append(f"drop {oid} {rem}")
            else:
                own = bids if side == "buy" else asks
                o = {"id": oid, "t": t, "q": rem, "px": limit}
                own.setdefault(limit, []).append(o); orders[oid] = (o, side)
                out.append(f"rest {oid} {rem}")
    for ln in case["stdin"].split("\n"):
        if ln == "": continue
        p = ln.split(" ")
        c = p[0]
        if c in ("buy", "sell") and len(p) in (4, 5) and "" not in p and num(p[2], 1, 1000000) and num(p[3], 1, 100000) and (len(p) == 4 or p[4] in ("ioc", "fok")):
            submit(c, p[1], int(p[2]), int(p[3]), p[4] if len(p) == 5 else "gtc")
        elif c in ("mbuy", "msell") and len(p) == 3 and "" not in p and num(p[2], 1, 1000000):
            submit("buy" if c == "mbuy" else "sell", p[1], int(p[2]), None, "mkt")
        elif c in ("cancel", "amend") and len(p) == (3 if c == "cancel" else 4) and "" not in p and num(p[2], 1, 10**7) and (c == "cancel" or num(p[3], 1, 1000000)):
            oid = int(p[2])
            if oid not in orders: err(f"no such order {oid}"); continue
            o, side = orders[oid]
            if o["t"] != p[1]: err("not owner"); continue
            book = bids if side == "buy" else asks
            lvl = book[o["px"]]
            if c == "cancel":
                lvl.remove(o); del orders[oid]
                if not lvl: del book[o["px"]]
                out.append(f"cancelled {oid} {o['q']}")
            else:
                nq = int(p[3])
                if nq > o["q"]:
                    lvl.remove(o); lvl.append(o)
                o["q"] = nq
                out.append(f"amended {oid} {nq}")
        elif c == "book" and len(p) == 1:
            for name, book in (("asks", asks), ("bids", bids)):
                out.append(name + ":")
                if not book: out.append("  (none)")
                for px in sorted(book, reverse=True):
                    out.append(f"  {px} {sum(o['q'] for o in book[px])} {len(book[px])}")
        elif c == "pos" and len(p) == 1:
            if not pos: out.append("(no traders)")
            for t in sorted(pos): out.append(f"{t} {pos[t][0]} {pos[t][1]}")
        else:
            err("bad command")
    return "".join(x + "\n" for x in out), (1 if errs else 0)
