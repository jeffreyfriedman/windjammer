def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["42", "-7", "abc", "99999999999"]), _c(["", "+5", "-", "1 2"])]
EDGE = [_c(["2147483647", "2147483648", "-2147483648", "-2147483649", "007", "-0", "+0", "--1", "1.0", " 1", "+", "000000000000000000000000000012"])]
POOL = ["0", "1", "-1", "+9", "12a", "", "x", "2147483647", "-2147483648", "2147483648", "99999999999999999999", "-", "+", "1_000", "3 4", "00", "-007"]


def rand(rng):
    return _c([rng.choice(POOL) if rng.random() < 0.7 else str(rng.randint(-3 * 10 ** 9, 3 * 10 ** 9)) for _ in range(rng.randint(1, 10))])


def solve(case):
    out, c = [], {"ok": 0, "invalid": 0, "overflow": 0, "empty": 0}
    for t in case["stdin"].split("\n")[:-1]:
        if t == "":
            out.append("error empty")
            c["empty"] += 1
            continue
        body = t[1:] if t[0] in "+-" else t
        if not body or not all("0" <= ch <= "9" for ch in body):
            out.append(f"error invalid {t}")
            c["invalid"] += 1
            continue
        v = int(t)
        if -2 ** 31 <= v < 2 ** 31:
            out.append(f"ok {v}")
            c["ok"] += 1
        else:
            out.append(f"error overflow {t}")
            c["overflow"] += 1
    out.append(f"summary ok={c['ok']} invalid={c['invalid']} overflow={c['overflow']} empty={c['empty']}")
    return "\n".join(out) + "\n"
