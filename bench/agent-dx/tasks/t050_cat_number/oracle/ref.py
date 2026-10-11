def _c(files, args, stdin=""):
    return {"stdin": stdin, "args": args, "files": files}


F = {"a": "one\ntwo\n", "b": "three\n\nfive", "e": ""}
SAMPLES = [_c(F, ["a", "b"]), _c(F, ["-n", "a", "b"])]
EDGE = [_c(F, ["-n"], "from stdin\nlines\n"), _c(F, ["e", "-n"]), _c(F, ["-n", "x", "a"]), _c(F, ["b"]), _c(F, ["a", "-n", "a", "a"]), _c({"big": "".join(f"l{i}\n" for i in range(1500))}, ["-n", "big"]), _c(F, [], "")]


def rand(rng):
    files = {f"f{i}": "".join(rng.choice(["x", "yy", ""]) + "\n" for _ in range(rng.randint(0, 6))) + rng.choice(["", "tail"]) for i in range(3)}
    args = [rng.choice(list(files) + ["nope"]) for _ in range(rng.randint(0, 3))]
    if rng.random() < 0.5:
        args.insert(rng.randint(0, len(args)), "-n")
    return _c(files, args, "in1\nin2\n" if not [a for a in args if a != "-n"] else "")


def solve(case):
    args = case["args"]
    num = "-n" in args
    names = [a for a in args if a != "-n"]
    out, n, bad = "", 0, False
    srcs = [("x", case["stdin"])] if not names else []
    for nm in names:
        if nm in case["files"]:
            srcs.append((nm, case["files"][nm]))
        else:
            srcs.append((nm, None))
    for nm, text in srcs:
        if text is None:
            out += f"cat: {nm}: No such file\n"
            bad = True
            continue
        if not num:
            out += text
            continue
        lines = text.split("\n")
        if lines[-1] == "":
            lines.pop()
        for ln in lines:
            n += 1
            out += f"{n:6d}\t{ln}\n"
    return out, (1 if bad else 0)
