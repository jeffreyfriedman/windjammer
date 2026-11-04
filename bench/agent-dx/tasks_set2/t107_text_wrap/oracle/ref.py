def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("10\nthe quick brown fox jumps over the lazy dog\n"), _c("5\nabcdefgh ij k\n")]
EDGE = [_c("8\n"), _c("0\nx\n"), _c("abc\nx\n"), _c("3\na  b\tc\n\n d\n"), _c("1\na b c\n"), _c("20\nexactly twenty chars!! and more\n")]
def rand(rng):
    ws = ["a", "bb", "ccc", "dddd", "eeeee", "ffffffffff"]
    text = "\n".join(" ".join(rng.choice(ws) for _ in range(rng.randint(0, 8))) for _ in range(rng.randint(1, 3)))
    return _c(f"{rng.randint(1, 14)}\n{text}\n")
def solve(case):
    first, _, rest = case["stdin"].partition("\n")
    if not first.isdigit() or int(first) == 0: return "error: bad width\n", 1
    w = int(first)
    lines, cur = [], ""
    for word in rest.split():
        if not cur: cur = word
        elif len(cur) + 1 + len(word) <= w: cur += " " + word
        else: lines.append(cur); cur = word
    if cur: lines.append(cur)
    return "".join(l + "\n" for l in lines)
