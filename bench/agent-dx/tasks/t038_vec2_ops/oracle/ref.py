def _c(rows):
    return {"stdin": "".join(f"{a} {b} {op} {c} {d}\n" for a, b, op, c, d in rows)}


SAMPLES = [_c([(1, 2, "+", 3, 4), (1, 2, "*", 3, 4), (1, 2, "x", 3, 4)]), _c([(1, 2, "-", 3, 5), (2, 3, "s", 4, 99), (1, 1, "==", 1, 1)])]
EDGE = [_c([(0, 0, "+", 0, 0), (-3, 4, "s", -1, 0), (10 ** 9, 10 ** 9, "*", 10 ** 9, 10 ** 9), (1, 2, "==", 2, 1), (-1, -1, "-", -1, -1)])]


def rand(rng):
    return _c([(rng.randint(-50, 50), rng.randint(-50, 50), rng.choice(["+", "-", "*", "x", "s", "=="]), rng.randint(-50, 50), rng.randint(-50, 50)) for _ in range(rng.randint(1, 10))])


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        ax, ay, op, bx, by = ln.split()
        ax, ay, bx, by = int(ax), int(ay), int(bx), int(by)
        if op == "+":
            out.append(f"({ax + bx}, {ay + by})")
        elif op == "-":
            out.append(f"({ax - bx}, {ay - by})")
        elif op == "*":
            out.append(str(ax * bx + ay * by))
        elif op == "x":
            out.append(str(ax * by - ay * bx))
        elif op == "s":
            out.append(f"({ax * bx}, {ay * bx})")
        else:
            out.append("true" if (ax, ay) == (bx, by) else "false")
    return "\n".join(out) + "\n"
