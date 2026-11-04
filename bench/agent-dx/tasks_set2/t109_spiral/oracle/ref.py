def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("3 3\n1 2 3\n4 5 6\n7 8 9\n"), _c("2 4\n1 2 3 4\n5 6 7 8\n")]
EDGE = [_c("1 1\n5\n"), _c("1 4\n1 2 3 4\n"), _c("4 1\n1\n2\n3\n4\n"), _c("2 2\n1 2\n"), _c("0 3\n"), _c("x y\n"), _c("2 2\n1 2\n3\n"), _c("3 2\n1 2\n3 4\n5 6\n"), _c("2 2\n-1 2\n3 -4\n")]
def rand(rng):
    r, c = rng.randint(1, 6), rng.randint(1, 6)
    rows = [" ".join(str(rng.randint(-9, 99)) for _ in range(c)) for _ in range(r)]
    return _c(f"{r} {c}\n" + "\n".join(rows) + "\n")
def solve(case):
    ls = case["stdin"].splitlines()
    try:
        r, c = map(int, ls[0].split(" "))
        assert 1 <= r <= 20 and 1 <= c <= 20 and len(ls) >= r + 1
        m = []
        for l in ls[1:r + 1]:
            row = list(map(int, l.split(" ")))
            assert len(row) == c
            m.append(row)
    except Exception:
        return "error\n", 1
    out, top, bot, lef, rig = [], 0, r - 1, 0, c - 1
    while top <= bot and lef <= rig:
        for j in range(lef, rig + 1): out.append(m[top][j])
        top += 1
        for i in range(top, bot + 1): out.append(m[i][rig])
        rig -= 1
        if top <= bot:
            for j in range(rig, lef - 1, -1): out.append(m[bot][j])
            bot -= 1
        if lef <= rig:
            for i in range(bot, top - 1, -1): out.append(m[i][lef])
            lef += 1
    return " ".join(map(str, out)) + "\n"
