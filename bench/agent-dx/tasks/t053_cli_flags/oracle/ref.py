def _c(args):
    return {"stdin": "", "args": args, "files": {}}


SAMPLES = [_c(["--name=bob", "-v", "a", "b"]), _c(["--count", "3", "-vv", "--dry-run", "x"])]
EDGE = [_c([]), _c(["--"]), _c(["--", "-v", "--name=x"]), _c(["-"]), _c(["--name"]), _c(["--count"]), _c(["--count=-1"]), _c(["--count=abc"]), _c(["--count", "007"]), _c(["--bogus"]), _c(["-x"]), _c(["--name", "--dry-run"]),
        _c(["-vvv", "-v"]), _c(["--name=a", "--name=b"]), _c(["--name="]), _c(["pos", "--", "--count", "9"]), _c(["-vx"]), _c(["--count=", "--bogus"]), _c(["--dry-run=1"])]


def rand(rng):
    toks = ["--name=n", "--name", "zed", "--count=5", "--count", "2", "-v", "-vv", "--dry-run", "pos", "-", "--", "--oops", "-q", "7", "--count=x"]
    return _c([rng.choice(toks) for _ in range(rng.randint(0, 7))])


def solve(case):
    a = case["args"]
    name, count, v, dry, pos = "anonymous", 1, 0, False, []
    i, rest = 0, False

    def cnt(x):
        if not (x.isascii() and x.isdigit()):
            return None
        return int(x)
    while i < len(a):
        x = a[i]
        i += 1
        if rest or x == "-" or not x.startswith("-"):
            pos.append(x)
        elif x == "--":
            rest = True
        elif x == "--dry-run":
            dry = True
        elif x.startswith("--name") and (x == "--name" or x.startswith("--name=")):
            if x == "--name":
                if i >= len(a):
                    return "error: missing value for --name\n", 2
                name = a[i]
                i += 1
            else:
                name = x[7:]
        elif x.startswith("--count") and (x == "--count" or x.startswith("--count=")):
            if x == "--count":
                if i >= len(a):
                    return "error: missing value for --count\n", 2
                val = a[i]
                i += 1
            else:
                val = x[8:]
            c = cnt(val)
            if c is None:
                return f"error: invalid count {val}\n", 2
            count = c
        elif not x.startswith("--") and set(x[1:]) == {"v"}:
            v += len(x) - 1
        else:
            return f"error: unknown option {x}\n", 2
    return f"name: {name}\ncount: {count}\nverbosity: {v}\ndry-run: {'true' if dry else 'false'}\npositional: {' '.join(pos)}\n"
