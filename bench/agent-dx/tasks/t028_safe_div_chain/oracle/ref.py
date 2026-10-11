MIN = -2 ** 63


def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["100 / 5 / 2", "7 / 0", "-9"]), _c(["10 / 3", "-10 / 3", "1 / 2 / 0"])]
EDGE = [_c([f"{MIN} / -1", f"{MIN} / 1", f"{MIN} / 2 / -1", "9223372036854775807 / -1", "5 / 1 / 0 / 0", "0 / 7", f"{MIN} / -2"])]


def rand(rng):
    lines = []
    for _ in range(rng.randint(2, 8)):
        k = rng.randint(1, 5)
        nums = [rng.choice([rng.randint(-10 ** 6, 10 ** 6), rng.randint(-10, 10), 0, -1, 1, MIN, 2 ** 63 - 1]) for _ in range(k)]
        lines.append(" / ".join(map(str, nums)))
    return _c(lines)


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        nums = [int(x) for x in ln.split(" / ")]
        v, err = nums[0], None
        for k, d in enumerate(nums[1:], 1):
            if d == 0:
                err = f"error: division by zero at step {k}"
                break
            if v == MIN and d == -1:
                err = f"error: overflow at step {k}"
                break
            q = abs(v) // abs(d)
            v = q if (v < 0) == (d < 0) else -q
        out.append(err or str(v))
    return "\n".join(out) + "\n"
