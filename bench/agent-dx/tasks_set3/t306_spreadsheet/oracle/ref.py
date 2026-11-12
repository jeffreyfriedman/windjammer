import re
def _c(stdin): return {"stdin": stdin}
EX = "set A1 10\nset A2 =A1*3\nset B1 =SUM(A1:A2)+MAX(1,A2)\nset A3 =B1\nget B1\nset A1 =A3\nset C1 =10/(A1-A1)\ndump\n"
SAMPLES = [
    _c(EX),
    _c("set A1 5\nset B2 hello\nset C3 =A1+B2\nget C3\nset B2 7\nget C3\nclear A1\nget C3\nget A1\nget Z9\nset A1\nbogus\n"),
]
EDGE = [
    _c("set A1 =65536*65536\nset A2 =A1*A1\nset A3 =A1*A1*0\nset A4 =0*A1*A1\nset A5 =SUM(A1:A1,A1)*A1\nget A2\ndump\n"),
    _c(""),
    _c("dump\nget A1\n"),
    _c("set A1 =A1\nset A1 =1+A1*2\nset B1 =A1\nset A1 =B1\nset A1 =SUM(B1:B1)\nset A1 =SUM(A1:C3)\nget A1\n"),
    _c("set A1 =C3\nset C3 =B2\nset B2 =SUM(A1:B1)\nset B1 =SUM(A1:A1)\nset A1 =MAX(B2,C3)\nset B1 =5\nset A1 =MIN(B1, 2)\nget A1\n"),
    _c("set A1 =7/2\nset A2 =-7/2\nset A3 =7/-2\nset A4 =-(7)/2\nset B1 =1/0\nset B2 =B1+1\nset B3 =1+B1\nset B4 =SUM(B1,A1)\nset B5 =SUM(A1:B5)\ndump\n"),
    _c("set A1 1000000\nset A2 =A1*A1\nset A3 =A2+1\nset A4 =A2-1\nset A5 =-A2\nset A6 =A5-1\ndump\n"),
    _c("set A1 999999999\nset A2 =A1*A1\nset A3 =A2*1000\nset A4 =A2+A3\nset A5 =SUM(A1,A1,A1)*1000\nset A6 =A5-A5\nset A7 =A3-A3\nset A8 =-A2\ndump\n"),
    _c("set A1 3\nset A2 x y\nset A3 =\nset A4 =A1+\nset A5 =FOO(A1)\nset A6 =SUM()\nset A7 =ABS(A1:A2)\nset A8 =A1:A2\nset A9 =SUM(A1:A2:A3)\nset A10 =(A1\nset A11 =A1)\nset A12 =a1\nset A13 =A01\nset A14 =1 2\nset B1 =SUM (A1 , 4 )\nset B2 =ABS(0-5)\nset B3 =ABS(A2)\nset B4 =ABS(1,2)\nset B5 =1234567890\ndump\n"),
    _c("set A1 1\nset A2 2\nset B1 =COUNT(A1:A5)\nset B2 =COUNT(A1:A5, 7, A1)\nset B3 =MIN(A3:A5)\nset B4 =MAX(A5:A3)\nset B5 =SUM(A3:A5)\nset C1 hi\nset C2 =C1\nset C3 =COUNT(C1:C2)\nset C4 =MIN(C1:C2, 4)\nset C5 =SUM(C2:C3)\ndump\n"),
    _c("set A1 -0\nset A2 --5\nset A3 -5\nset A4 +5\nset A5 1234567890\nset A6 123456789\nset A7 -123456789\nset A8 =--3\nset A9 =-A3\nset A10 =- -A3\nget A1\ndump\n"),
    _c("set A1 =B1\nset B1 =C1\nset C1 =D1\nset D1 =1\nget A1\nset D1 =A1\nset D1 =E1\nset E1 =A1\nset E1 =2\nget A1\nclear D1\nget A1\nset A1 =SUM(B1:D1)\nset A1 =SUM(A1:A1)\n"),
    _c("set A1 1\nset  A2 2\nset A3  3\nget A1 \nget\nclear\nclear A1 B1\nset J20 =J20\nset J20 5\nset K1 2\nset A21 2\nget J20\nclear J20\nget J20\ndump x\n"),
]
def rand(rng):
    cols, rows = "ABCD", 4
    cells = [f"{c}{r}" for c in cols for r in range(1, rows + 1)]
    def build(target):
        # references mostly point at cells that sort earlier than the target, so cycles are the exception
        ti = cells.index(target)
        def cell():
            if rng.random() < 0.93 and ti > 0: return cells[rng.randrange(ti)]
            return rng.choice(cells)
        def expr(d=0):
            r = rng.random()
            if d > 2 or r < 0.25: return rng.choice([cell(), str(rng.randint(0, 20)), cell()])
            if r < 0.55: return f"{expr(d+1)}{rng.choice('+-*/')}{expr(d+1)}"
            if r < 0.62: return f"({expr(d+1)})"
            if r < 0.67: return f"-{expr(d+1)}"
            f = rng.choice(["SUM", "MIN", "MAX", "COUNT", "SUM", "ABS"])
            if f == "ABS": return f"ABS({expr(d+1)})"
            args = []
            for _ in range(rng.randint(1, 3)):
                if rng.random() < 0.6: args.append(f"{cell()}:{cell()}")
                else: args.append(expr(d+1))
            return f"{f}(" + ",".join(args) + ")"
        return expr()
    lines = []
    for _ in range(rng.randint(30, 70)):
        r = rng.random()
        t = rng.choice(cells)
        if r < 0.22: lines.append(f"set {t} {rng.choice([rng.randint(-30, 50), rng.randint(-30, 50), 999999, 1000000])}")
        elif r < 0.3: lines.append(f"set {t} {rng.choice(['x', 'two words', 'n1'])}")
        elif r < 0.68: lines.append(f"set {t} ={build(t)}")
        elif r < 0.76: lines.append(f"clear {rng.choice(cells)}")
        elif r < 0.93: lines.append(f"get {rng.choice(cells)}")
        elif r < 0.97: lines.append("dump")
        else: lines.append(rng.choice(["set A1 =", "set E1 3", "get", "set A1 =SUM(A1)", "nope", "set B2 =1000000*1000000"]))
    lines.append("dump")
    return _c("\n".join(lines) + "\n")

CELL_RE = re.compile(r"^[A-J](?:[1-9]|1[0-9]|20)$")
LIM = 10**12
class Bad(Exception): pass
def tokenize(s):
    toks, i = [], 0
    while i < len(s):
        ch = s[i]
        if ch == " ": i += 1
        elif ch in "+-*/(),:": toks.append(ch); i += 1
        elif ch.isascii() and ch.isdigit():
            j = i
            while j < len(s) and s[j].isascii() and s[j].isdigit(): j += 1
            if j - i > 9: raise Bad()
            toks.append(("n", int(s[i:j]))); i = j
        elif ch.isascii() and ch.isalpha():
            j = i
            while j < len(s) and s[j].isascii() and s[j].isalnum(): j += 1
            toks.append(("w", s[i:j])); i = j
        else: raise Bad()
    return toks
class P:
    def __init__(self, t): self.t, self.i = t, 0
    def peek(self): return self.t[self.i] if self.i < len(self.t) else None
    def eat(self, x=None):
        v = self.peek()
        if v is None or (x is not None and v != x): raise Bad()
        self.i += 1; return v
    def expr(self):
        a = self.term()
        while self.peek() in ("+", "-"):
            op = self.eat(); a = ("bin", op, a, self.term())
        return a
    def term(self):
        a = self.unary()
        while self.peek() in ("*", "/"):
            op = self.eat(); a = ("bin", op, a, self.unary())
        return a
    def unary(self):
        if self.peek() == "-": self.eat(); return ("neg", self.unary())
        return self.primary()
    def primary(self):
        v = self.peek()
        if v is None: raise Bad()
        if isinstance(v, tuple) and v[0] == "n": self.eat(); return ("num", v[1])
        if v == "(":
            self.eat(); e = self.expr(); self.eat(")"); return e
        if isinstance(v, tuple) and v[0] == "w":
            self.eat()
            if self.peek() == "(":
                if v[1] not in ("SUM", "MIN", "MAX", "COUNT", "ABS"): raise Bad()
                self.eat(); args = [self.arg()]
                while self.peek() == ",": self.eat(); args.append(self.arg())
                self.eat(")")
                if v[1] == "ABS" and (len(args) != 1 or args[0][0] == "range"): raise Bad()
                return ("fn", v[1], args)
            if not CELL_RE.match(v[1]): raise Bad()
            return ("ref", v[1])
        raise Bad()
    def arg(self):
        v = self.peek(); n = self.t[self.i + 1] if self.i + 1 < len(self.t) else None
        if isinstance(v, tuple) and v[0] == "w" and n == ":":
            self.eat(); self.eat(); w = self.eat()
            if not (isinstance(w, tuple) and w[0] == "w") or not CELL_RE.match(v[1]) or not CELL_RE.match(w[1]): raise Bad()
            return ("range", v[1], w[1])
        return self.expr()
def parse_formula(s):
    p = P(tokenize(s)); e = p.expr()
    if p.peek() is not None: raise Bad()
    return e
def key(c): return (c[0], int(c[1:]))
def members(a, b):
    c1, c2 = sorted([a[0], b[0]]); r1, r2 = sorted([int(a[1:]), int(b[1:])])
    return [f"{chr(c)}{r}" for r in range(r1, r2 + 1) for c in range(ord(c1), ord(c2) + 1)]
def deps(ast):
    out = set()
    def w(n):
        if n[0] == "ref": out.add(n[1])
        elif n[0] == "range": out.update(members(n[1], n[2]))
        elif n[0] == "bin": w(n[2]); w(n[3])
        elif n[0] == "neg": w(n[1])
        elif n[0] == "fn":
            for a in n[2]: w(a)
    w(ast)
    return sorted(out, key=key)
def solve(case):
    sheet, out = {}, []
    def cellval(c):
        x = sheet.get(c)
        if x is None: return ("n", 0)
        if x[0] == "num": return ("n", x[1])
        if x[0] == "text": return ("t", x[1])
        return ev(x[1])
    def chk(v): return ("e", "#OVERFLOW") if abs(v) > LIM else ("n", v)
    def ev(n):
        k = n[0]
        if k == "num": return ("n", n[1])
        if k == "ref":
            v = cellval(n[1])
            return ("e", "#VALUE") if v[0] == "t" else v
        if k == "neg":
            v = ev(n[1]); return v if v[0] == "e" else ("n", -v[1])
        if k == "bin":
            a = ev(n[2])
            if a[0] == "e": return a
            b = ev(n[3])
            if b[0] == "e": return b
            if n[1] == "+": return chk(a[1] + b[1])
            if n[1] == "-": return chk(a[1] - b[1])
            if n[1] == "*": return chk(a[1] * b[1])
            if b[1] == 0: return ("e", "#DIV0")
            q = abs(a[1]) // abs(b[1])
            return chk(q if (a[1] < 0) == (b[1] < 0) else -q)
        nums = []
        for a in n[2]:
            if a[0] == "range":
                for m in members(a[1], a[2]):
                    x = sheet.get(m)
                    if x is None or x[0] == "text": continue
                    v = cellval(m)
                    if v[0] == "e": return v
                    nums.append(v[1])
            else:
                v = ev(a)
                if v[0] == "e": return v
                nums.append(v[1])
        f = n[1]
        if f == "ABS": return ("n", abs(nums[0]))
        if f == "COUNT": return ("n", len(nums))
        if f == "SUM":
            t = 0
            for x in nums:
                t += x
                if abs(t) > LIM: return ("e", "#OVERFLOW")
            return ("n", t)
        if not nums: return ("e", "#VALUE")
        return ("n", min(nums) if f == "MIN" else max(nums))
    def show(c):
        x = sheet.get(c)
        if x is None: return "(empty)"
        if x[0] == "text": return x[1]
        v = cellval(c)
        return v[1] if v[0] == "e" else str(v[1])
    def cycle(x, ast):
        seen = set()
        def dfs(c, path):
            for d in (deps(ast) if c == x else deps(sheet[c][1]) if c in sheet and sheet[c][0] == "f" else []):
                if d == x: return path + [x]
                if d in seen: continue
                seen.add(d)
                r = dfs(d, path + [d])
                if r: return r
            return None
        return dfs(x, [x])
    for ln in case["stdin"].split("\n"):
        if ln == "": continue
        p = ln.split(" ")
        c = p[0]
        if c == "set" and len(p) >= 3 and CELL_RE.match(p[1]) and ln.split(" ", 2)[2] != "":
            cell, content = p[1], ln.split(" ", 2)[2]
            if content.startswith("="):
                try: ast = parse_formula(content[1:])
                except Bad: out.append("error: bad formula"); continue
                cy = cycle(cell, ast)
                if cy: out.append("error: cycle " + " -> ".join(cy)); continue
                sheet[cell] = ("f", ast)
            elif re.fullmatch(r"-?[0-9]{1,9}", content):
                sheet[cell] = ("num", int(content))
            else: sheet[cell] = ("text", content)
        elif c == "clear" and len(p) == 2 and CELL_RE.match(p[1]): sheet.pop(p[1], None)
        elif c == "get" and len(p) == 2 and CELL_RE.match(p[1]): out.append(f"{p[1]} = {show(p[1])}")
        elif c == "dump" and len(p) == 1:
            if not sheet: out.append("(empty sheet)")
            for k in sorted(sheet, key=key): out.append(f"{k} = {show(k)}")
        else: out.append("error: bad command")
    return "".join(x + "\n" for x in out)
