def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("1 + 2 * 3\n(1 + 2) * 3\n2 ^ 3 ^ 2\n"), _c("10 - 4 - 3\n8 / 4 / 2\n")]
EDGE = [_c(""), _c("1 +\n"), _c("(1\n1)\n"), _c("1 2\n"), _c("()\n"), _c("a + 1\n"), _c("  42  \n"), _c("((7))\n"), _c("-1 + 2\n"), _c("1+2*3-4/5^6\n")]
def rand(rng):
    def expr(d):
        if d == 0 or rng.random() < 0.3: return str(rng.randint(0, 99))
        if rng.random() < 0.2: return "(" + expr(d - 1) + ")"
        return expr(d - 1) + rng.choice([" + ", "-", " * ", "/", "^"]) + expr(d - 1)
    ls = [expr(3) for _ in range(rng.randint(1, 4))]
    if rng.random() < 0.3: ls.append(rng.choice(["1 +", "(2", "3 4", "x"]))
    return _c("\n".join(ls) + "\n")
PREC = {"+": 1, "-": 1, "*": 2, "/": 2, "^": 3}
def solve(case):
    out = []
    for ln in case["stdin"].splitlines():
        if not ln.strip(): continue
        toks, i, ok = [], 0, True
        while i < len(ln) and ok:
            c = ln[i]
            if c == " ": i += 1
            elif c.isdigit() and c.isascii():
                j = i
                while j < len(ln) and ln[j].isdigit() and ln[j].isascii(): j += 1
                toks.append(ln[i:j]); i = j
            elif c in "+-*/^()": toks.append(c); i += 1
            else: ok = False
        if not ok or not toks: out.append("error\n"); continue
        res, st, expect_operand = [], [], True
        for t in toks:
            if t.isdigit():
                if not expect_operand: ok = False; break
                res.append(t); expect_operand = False
            elif t == "(":
                if not expect_operand: ok = False; break
                st.append(t)
            elif t == ")":
                if expect_operand: ok = False; break
                while st and st[-1] != "(": res.append(st.pop())
                if not st: ok = False; break
                st.pop()
            else:
                if expect_operand: ok = False; break
                while st and st[-1] != "(" and (PREC[st[-1]] > PREC[t] or (PREC[st[-1]] == PREC[t] and t != "^")): res.append(st.pop())
                st.append(t); expect_operand = True
        if ok and expect_operand: ok = False
        if ok:
            while st:
                if st[-1] == "(": ok = False; break
                res.append(st.pop())
        out.append((" ".join(res) if ok else "error") + "\n")
    return "".join(out)
