def _c(files, args, stdin=""):
    return {"stdin": stdin, "args": args, "files": files}


SAMPLES = [_c({"a.txt": "hello world\nfoo\n"}, ["a.txt"]), _c({"x": "one two\n", "y": "three\n\nfour five six\n"}, ["x", "y"])]
EDGE = [_c({}, [], "stdin text\nline two\n"), _c({}, ["nope.txt"]), _c({"e": ""}, ["e", "e"]), _c({"n": "no newline at end"}, ["n", "missing", "n"]), _c({"t": "a\tb  c\r\nd\n\n\n"}, ["t"]), _c({}, [], "")]


def rand(rng):
    files = {}
    for i in range(rng.randint(0, 4)):
        files[f"f{i}.txt"] = "".join(rng.choice(["ab", "cde", " ", "\n", "\t", "x y", "\n"]) for _ in range(rng.randint(0, 20)))
    args = [rng.choice(list(files) + ["gone.txt"]) for _ in range(rng.randint(1, 4))] if files else []
    return _c(files, args, "".join(rng.choice(["w ", "\n", "zz "]) for _ in range(5)) if not args else "")


def _cnt(s):
    return s.count("\n"), len(s.split()), len(s.encode())


def solve(case):
    if not case["args"]:
        l, w, b = _cnt(case["stdin"])
        return f"{l} {w} {b}\n"
    out, tot, bad = [], [0, 0, 0], False
    for a in case["args"]:
        if a not in case["files"]:
            out.append(f"wc: {a}: No such file")
            bad = True
            continue
        l, w, b = _cnt(case["files"][a])
        tot = [tot[0] + l, tot[1] + w, tot[2] + b]
        out.append(f"{l} {w} {b} {a}")
    if len(case["args"]) > 1:
        out.append(f"{tot[0]} {tot[1]} {tot[2]} total")
    return "\n".join(out) + "\n", (1 if bad else 0)
