def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["3 4 +", "5 1 2 + 4 * + 3 -"]), _c(["1 0 /", "+", "1 2", "2 x +"])]
EDGE = [_c(["-7 2 /", "7 -2 /", "-7 -2 /", "0 5 -", "1000000000 1000000000 *", "", "5", "-3"]),
        _c(["1 2 3", "1 +", "/", "4 0 0 / +", "1 2 + +"])]


def rand(rng):
    lines = []
    for _ in range(rng.randint(1, 6)):
        toks, depth = [], 0
        for _ in range(rng.randint(1, 9)):
            if depth >= 2 and rng.random() < 0.5:
                toks.append(rng.choice("+-*/"))
                depth -= 1
            else:
                toks.append(str(rng.randint(-20, 20)))
                depth += 1
        if rng.random() < 0.15:
            toks.append(rng.choice(["+", "abc", "1.5", "/"]))
        lines.append(" ".join(toks))
    return _c(lines)


def solve(case):
    out = []
    for ln in case["stdin"].split("\n"):
        if not ln.strip():
            continue
        st, err = [], None
        for t in ln.split(" "):
            if t in ("+", "-", "*", "/"):
                if len(st) < 2:
                    err = "underflow"
                    break
                b, a = st.pop(), st.pop()
                if t == "+":
                    st.append(a + b)
                elif t == "-":
                    st.append(a - b)
                elif t == "*":
                    st.append(a * b)
                else:
                    if b == 0:
                        err = "divzero"
                        break
                    q = abs(a) // abs(b)
                    st.append(q if (a < 0) == (b < 0) else -q)
            else:
                try:
                    if not (t.lstrip("-").isdigit() and t.count("-") <= 1 and t.rfind("-") <= 0):
                        raise ValueError
                    st.append(int(t))
                except ValueError:
                    err = "token"
                    break
        if err is None and len(st) != 1:
            err = "leftover"
        out.append(f"error: {err}" if err else str(st[0]))
    return "\n".join(out) + "\n"
