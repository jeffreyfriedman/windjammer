def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["babad", "cbbd"]), _c(["aaa", "", "abc"])]
EDGE = [_c(["a" * 200, "ab" * 100, "abcdefghijklmnopqrstuvwxyz", "racecar", "abacdfgdcaba", "z"])]


def rand(rng):
    return _c(["".join(rng.choice("ab" if rng.random() < 0.7 else "abc") for _ in range(rng.randint(0, 40))) for _ in range(rng.randint(1, 6))])


def solve(case):
    out = []
    for s in case["stdin"].split("\n")[:-1]:
        n, best, cnt = len(s), (0, 0), 0
        for i in range(n):
            for j in range(i, n):
                t = s[i:j + 1]
                if t == t[::-1]:
                    cnt += 1
                    if j - i + 1 > best[1] - best[0]:
                        best = (i, j + 1)
        sub = s[best[0]:best[1]]
        out.append(f"{len(sub)} {sub} {cnt}" if sub else "0 - 0")
    return "\n".join(out) + "\n"
