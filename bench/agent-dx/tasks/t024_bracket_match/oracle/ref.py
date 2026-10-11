def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["()", "([{}])", "(]"]), _c(["((", "a(b)c)", ""])]
EDGE = [_c(["{[()]}[", "]", "([)]", "(()", "no brackets here", "(a[b{c}d]e)f)", "{{{{"])]


def rand(rng):
    return _c(["".join(rng.choice("()[]{}ab ") for _ in range(rng.randint(0, 14))) for _ in range(rng.randint(3, 10))] +
              ["".join(c for c in "([{" ) ])


def solve(case):
    pairs = {")": "(", "]": "[", "}": "{"}
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        st, bad = [], None
        for i, ch in enumerate(ln, 1):
            if ch in "([{":
                st.append((ch, i))
            elif ch in pairs:
                if not st or st[-1][0] != pairs[ch]:
                    bad = i
                    break
                st.pop()
        if bad is None and st:
            bad = st[-1][1]
        out.append("ok" if bad is None else f"error {bad}")
    return "\n".join(out) + "\n"
