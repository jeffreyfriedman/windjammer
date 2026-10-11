def _c(files, args):
    return {"stdin": "", "args": args, "files": files}


F = {"a.txt": "Hello World\nfoo bar\nhello again\n", "b.txt": "nothing here\nWORLD of foo\n"}
SAMPLES = [_c(F, ["hello", "a.txt"]), _c(F, ["-i", "-n", "hello", "a.txt", "b.txt"])]
EDGE = [_c(F, ["-c", "foo", "a.txt", "b.txt"]), _c(F, ["-v", "-c", "o", "a.txt"]), _c(F, ["zzz", "a.txt"]), _c(F, ["foo", "missing", "a.txt"]), _c(F, ["a.txt", "a"]),
        _c({"e": ""}, ["x", "e"]), _c(F, ["-n", "-v", "-i", "world", "b.txt", "a.txt"]), _c({"d": "x\n\ny\n"}, ["", "d", "-n"]), _c({"nl": "a\nb"}, ["b", "nl", "-n"])]


def rand(rng):
    words = ["foo", "Bar", "baz", "FOO", "x", ""]
    files = {f"f{i}": "".join(" ".join(rng.choice(words) for _ in range(rng.randint(0, 3))) + "\n" for _ in range(rng.randint(0, 5))) for i in range(rng.randint(1, 3))}
    args = [rng.choice(["foo", "ba", "x", "o"])] + list(rng.sample(list(files), rng.randint(1, len(files))))
    if rng.random() < 0.2:
        args.append("ghost")
    for f in rng.sample(["-i", "-n", "-c", "-v"], rng.randint(0, 3)):
        args.insert(rng.randint(0, len(args)), f)
    # pattern must be the first non-flag argument
    return _c(files, args)


def solve(case):
    args = case["args"]
    flags = {a for a in args if a in ("-i", "-n", "-c", "-v")}
    pos = [a for a in args if a not in ("-i", "-n", "-c", "-v")]
    pat, files = pos[0], pos[1:]
    out, matched, status = [], 0, 0
    multi = len(files) > 1
    for f in files:
        if f not in case["files"]:
            out.append(f"grep: {f}: No such file")
            status = 2
            continue
        text = case["files"][f]
        lines = text.split("\n")
        if lines and lines[-1] == "":
            lines.pop()
        cnt = 0
        for i, ln in enumerate(lines, 1):
            hit = (pat.lower() in ln.lower()) if "-i" in flags else (pat in ln)
            if "-v" in flags:
                hit = not hit
            if hit:
                cnt += 1
                if "-c" not in flags:
                    pre = (f"{f}:" if multi else "") + (f"{i}:" if "-n" in flags else "")
                    out.append(pre + ln)
        matched += cnt
        if "-c" in flags:
            out.append((f"{f}:" if multi else "") + str(cnt))
    if status == 0:
        status = 0 if matched else 1
    return "".join(l + "\n" for l in out), status
