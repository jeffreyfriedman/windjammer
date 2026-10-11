def _c(files, args):
    return {"stdin": "", "args": args, "files": files}


F = {"d.csv": "name,qty,price\nbolt,10,3\nnut,25,1\nscrew,7,12\n"}
SAMPLES = [_c(F, ["d.csv", "qty", "sum"]), _c(F, ["d.csv", "price", "max"])]
EDGE = [_c(F, ["d.csv", "name", "count"]), _c(F, ["d.csv", "name", "sum"]), _c(F, ["d.csv", "nope", "sum"]), _c(F, ["d.csv", "qty", "pow"]), _c(F, ["gone.csv", "qty", "sum"]),
        _c({"h.csv": "a,b\n"}, ["h.csv", "a", "sum"]), _c({"h.csv": "a,b\n"}, ["h.csv", "a", "mean"]), _c({"h.csv": "a,b\n"}, ["h.csv", "a", "count"]), _c({"s.csv": "a,b\n1\n2,3\n"}, ["s.csv", "b", "sum"]), _c({"s.csv": "a,b\n1\n2,3\n"}, ["s.csv", "b", "count"]),
        _c(F, ["d.csv", "qty", "mean"]), _c(F, ["d.csv", "qty", "min"]), _c({"n.csv": "a\n-1\n"}, ["n.csv", "a", "sum"]), _c({"n.csv": "a\n5"}, ["n.csv", "a", "sum"])]


def rand(rng):
    rows = [[str(rng.randint(0, 1000)) if rng.random() < 0.95 else rng.choice(["x", "-2", ""]) for _ in range(3)] for _ in range(rng.randint(0, 8))]
    t = "a,b,c\n" + "".join(",".join(r) + "\n" for r in rows)
    return _c({"f.csv": t}, ["f.csv", rng.choice(["a", "b", "c", "zz"]), rng.choice(["sum", "min", "max", "count", "mean"])])


def solve(case):
    f, col, op = case["args"]
    if f not in case["files"]:
        return f"error: cannot read {f}\n", 1
    ls = case["files"][f].split("\n")
    if ls and ls[-1] == "":
        ls.pop()
    hdr = ls[0].split(",")
    if col not in hdr:
        return f"error: no such column {col}\n", 2
    if op not in ("sum", "min", "max", "count", "mean"):
        return f"error: unknown op {op}\n", 2
    ci = hdr.index(col)
    vals = []
    for n, l in enumerate(ls[1:], 2):
        p = l.split(",")
        v = p[ci] if ci < len(p) else ""
        vals.append((n, v))
    if op == "count":
        return f"{len(vals)}\n"
    nums = []
    for n, v in vals:
        if not (v.isascii() and v.isdigit()):
            return f"error: bad value {v} on line {n}\n", 3
        nums.append(int(v))
    if not nums:
        return "0\n" if op == "sum" else "n/a\n"
    r = {"sum": sum(nums), "min": min(nums), "max": max(nums), "mean": sum(nums) // len(nums)}[op]
    return f"{r}\n"
