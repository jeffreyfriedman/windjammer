def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("pen 3 150 plain\nink 10 999 bulk\ncap 4 2000 member\n"), _c("odd 1 5 gold\nx 0 100 plain\n")]
EDGE = [_c(""), _c("a 9 100 bulk\nb 10 100 bulk\n\nc 2 99 member\nd 3 100 member\n"), _c("m 3 10 member\n"), _c("z 1000 1999 bulk\n")]
def rand(rng):
    return _c("".join(f"{rng.choice(['pen','ink','cap','box'])} {rng.randint(0,15)} {rng.choice([0,5,99,150,1999])} {rng.choice(['plain','bulk','member','plain','vip'])}\n" for _ in range(rng.randint(1, 6))))
def _m(c): return f"${c // 100}.{c % 100:02d}"
def solve(case):
    out, tot = [], 0
    for ln in case["stdin"].splitlines():
        if not ln.strip(): continue
        name, q, u, k = ln.split()
        q, u = int(q), int(u)
        base = q * u
        if k == "plain": c = base
        elif k == "bulk": c = base - (base // 10 if q >= 10 else 0)
        elif k == "member":
            c = base - base * 5 // 100
            if q >= 3: c = max(0, c - 100)
        else:
            out.append(f"{name} invalid kind\n"); continue
        out.append(f"{name} {_m(c)}\n"); tot += c
    return "".join(out) + f"total {_m(tot)}\n"
