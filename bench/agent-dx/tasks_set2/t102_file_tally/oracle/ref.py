def _c(files, args): return {"stdin": "", "args": args, "files": files}
SAMPLES = [_c({"a.txt": "hello world\nfoo\n"}, ["a.txt"]), _c({"x": "one two\n", "y": "three\n\nfour five six\n"}, ["x", "gone", "y"])]
EDGE = [_c({}, []), _c({}, ["nothing"]), _c({"e": ""}, ["e", "e"]), _c({"n": "no newline"}, ["n"]), _c({"t": "a\tb  c\r\nd\n\n\n"}, ["t"])]
def rand(rng):
    files = {f"f{i}.txt": "".join(rng.choice(["ab ", "cde", " ", "\n", "\t", "x y\n"]) for _ in range(rng.randint(0, 12))) for i in range(rng.randint(0, 3))}
    return _c(files, [rng.choice(list(files) + ["gone.txt"]) for _ in range(rng.randint(1, 4))])
def solve(case):
    if not case["args"]:
        return "usage: tally FILE...\n", 2
    out, rep, bad = [], [], False
    for a in case["args"]:
        if a in case["files"]:
            s = case["files"][a]
            l, w = s.count("\n"), len(s.split())
            out.append(f"{a}: {l} lines, {w} words\n"); rep.append(f"{a} {l} {w}\n")
        else:
            out.append(f"{a}: cannot read\n"); rep.append(f"{a} -\n"); bad = True
    text = "".join(out) + "report:\n" + "".join(rep)
    return (text, 1) if bad else text
