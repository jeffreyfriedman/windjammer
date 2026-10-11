def _c(files, args):
    return {"stdin": "", "args": args, "files": files}


F = {"in.txt": "banana\nApple\ncherry\napple\nbanana\n  pad  \n"}
SAMPLES = [_c(F, ["in.txt", "out.txt", "upper"]), _c(F, ["in.txt", "o", "sort"])]
EDGE = [_c(F, ["in.txt", "o", "dedupe"]), _c(F, ["in.txt", "o", "trim"]), _c(F, ["in.txt", "o", "number"]), _c(F, ["in.txt", "o", "reverse"]), _c(F, ["nope", "o", "upper"]), _c(F, ["in.txt", "o", "weird"]),
        _c(F, ["in.txt"]), _c({"e": ""}, ["e", "o", "sort"]), _c({"nl": "a\n\nb"}, ["nl", "nl", "reverse"]), _c(F, ["in.txt", "in.txt", "upper"])]


def rand(rng):
    ls = [rng.choice(["a", "B", " c ", "d d", "", "a", "zz"]) for _ in range(rng.randint(0, 10))]
    f = {"in": "\n".join(ls) + ("\n" if ls and rng.random() < 0.8 else "")}
    return _c(f, ["in", "out", rng.choice(["upper", "reverse", "sort", "dedupe", "trim", "number"])])


def solve(case):
    a = case["args"]
    if len(a) != 3:
        return "usage: transform INPUT OUTPUT MODE\n", 2
    inp, outp, mode = a
    if inp not in case["files"]:
        return f"error: cannot read {inp}\n", 1
    if mode not in ("upper", "reverse", "sort", "dedupe", "trim", "number"):
        return f"error: unknown mode {mode}\n", 2
    t = case["files"][inp]
    ls = t.split("\n")
    if ls and ls[-1] == "":
        ls.pop()
    if mode == "upper":
        ls = [l.upper() for l in ls]
    elif mode == "reverse":
        ls = ls[::-1]
    elif mode == "sort":
        ls = sorted(ls, key=lambda s: s.encode())
    elif mode == "dedupe":
        seen, o = set(), []
        for l in ls:
            if l not in seen:
                seen.add(l)
                o.append(l)
        ls = o
    elif mode == "trim":
        ls = [l.strip(" ") for l in ls]
    else:
        ls = [f"{i}:{l}" for i, l in enumerate(ls, 1)]
    return f"wrote {len(ls)} lines to {outp}\n" + "".join(l + "\n" for l in ls)
