import sys
import threading

def _c(stdin): return {"stdin": stdin}
SAMPLES = [
    _c("(define make-counter\n  (lambda ()\n    (let ((n 0))\n      (lambda () (set! n (+ n 1)) n))))\n(define c (make-counter))\n(c)\n(print (c))\n(print (c))\n(print (/ 1 0))\n"),
    _c("; sum of squares\n(define sq (lambda (x) (* x x)))\n(define total 0)\n(define i 1)\n(while (<= i 5)\n  (set! total (+ total (sq i)))\n  (set! i (+ i 1)))\n(print total)\n(print sq)\n(print (< total 50))\n"),
]
EDGE = [
    _c(""),
    _c("(print 1)\n(print (+ 1 2)\n"),
    _c("(print 1)\n(print 2))\n(print 3)\n"),
    _c("(print 1)\n\n(print 1234567890123456)\n"),
    _c("(define f (lambda (n) (if (= n 0) 0 (+ 1 (f (- n 1))))))\n(print (f 99))\n(print (f 100))\n"),
    _c("(print (* 1000000 1000000000))\n(print (+ (* 1000000 1000000000) 1))\n"),
    _c("(print (* 4294967296 4294967296))\n"),
    _c("(print (- 0 (* 999999999999999 1)))\n(print (- (* 999999999999999 1) 1 1))\n(print (- 5))\n(print (+))\n(print (*))\n(print (% -7 2))\n(print (/ -7 2))\n(print (% 7 -2))\n(print (/ 7 -2))\n(print (% 1 0))\n"),
    _c("(define x 1)\n(print (set! x 5))\n(print x)\n(print (define y 2))\n(print (begin))\n(print (if #f 1))\n(print (and))\n(print (or))\n(print (and #t #f 7))\n(print (or #f #t 7))\n(print (or 1))\n"),
    _c("(define x 10)\n(define f (lambda (x) (set! x (+ x 1)) x))\n(print (f 1))\n(print x)\n(define g (lambda () (set! x (+ x 1))))\n(g)\n(g)\n(print x)\n(let ((x 100)) (g) (print x))\n(print x)\n"),
    _c("(define a 1)\n(define b (lambda () a))\n(define a 2)\n(print (b))\n(let ((a 5) (c a)) (print (+ a c)))\n(let ((a 5) (a 6)) a)\n"),
    _c("(define loop (lambda () (loop)))\n(print 1)\n(loop)\n"),
    _c("(define x 0)\n(while #t\n  (set! x (+ x 1)))\n"),
    _c("(print (undefined-thing 1))\n"),
    _c("(print\n (+ 1\n    zzz))\n"),
    _c("(define f (lambda (a b) a))\n(print (f 1))\n"),
    _c("(define f (lambda (a b) a))\n(print\n  (f 1\n     2 3))\n"),
    _c("(print (5 3))\n"),
    _c("(print (+ 1 #t))\n"),
    _c("(if 1 2 3)\n"),
    _c("(define if 3)\n"),
    _c("(lambda (x x) x)\n"),
    _c("(lambda (+) 1)\n"),
    _c("(lambda (x))\n"),
    _c("(let ((x)) x)\n"),
    _c("(let ((x 1 2)) x)\n"),
    _c("(print (= 1 2 3))\n"),
    _c("(print (/ 1))\n"),
    _c("()\n"),
    _c("(print (()))\n"),
    _c("(define f (lambda () 1))\n(print (f))\n(print ((lambda (a) (* a a)) 7))\n(print (let () 4))\n(print (let ((f (lambda (n) (+ n 1)))) (f 1)))\n"),
    _c("(define compose (lambda (f g) (lambda (x) (f (g x)))))\n(define inc (lambda (x) (+ x 1)))\n(define dbl (lambda (x) (* x 2)))\n(print ((compose inc dbl) 5))\n(print ((compose dbl inc) 5))\n"),
    _c("(define x -5)\n(print -x)\n"),
    _c("(print 1) ; trailing\n;; only comment\n(print #t)(print #f)(print (not #t))\n(not 1)\n"),
    _c("(print (while #f 1))\n(while #f)\n"),
    _c("(print 1)\n(set! q 1)\n"),
    _c("(define x 5)\n(x)\n"),
]
def rand(rng):
    out = []
    n = [0]
    def nm(p): n[0] += 1; return f"{p}{n[0]}"
    def val(): return str(rng.randint(-9, 30))
    def snip():
        r = 11 if rng.random() < 0.05 else rng.randint(0, 10)
        if r == 0:
            mk, c, d = nm("mk"), nm("c"), nm("c")
            s = f"(define {mk} (lambda (start step)\n  (let ((n start))\n    (lambda () (set! n (+ n step)) n))))\n(define {c} ({mk} {val()} {val()}))\n(define {d} ({mk} {val()} {rng.randint(1, 5)}))\n"
            for _ in range(rng.randint(2, 5)): s += f"(print ({rng.choice([c, d])}))\n"
            return s
        if r == 1:
            x, f = nm("x"), nm("bump")
            s = f"(define {x} {val()})\n(define {f} (lambda (k) (set! {x} (+ {x} k)) {x}))\n"
            for _ in range(rng.randint(2, 4)): s += f"(print ({f} {val()}))\n"
            return s + f"(print {x})\n"
        if r == 2:
            f = nm("fact")
            return f"(define {f} (lambda (n) (if (<= n 1) 1 (* n ({f} (- n 1))))))\n(print ({f} {rng.choice([rng.randint(0, 17), rng.randint(0, 17), rng.randint(0, 22)])}))\n"
        if r == 3:
            f = nm("fib")
            return f"(define {f} (lambda (n) (if (< n 2) n (+ ({f} (- n 1)) ({f} (- n 2))))))\n(print ({f} {rng.randint(0, 12)}))\n"
        if r == 4:
            t, i = nm("t"), nm("i")
            return f"(define {t} 0)\n(define {i} {rng.randint(0, 3)})\n(while (< {i} {rng.randint(3, 12)})\n  (set! {t} (+ {t} (* {i} {rng.randint(1, 4)})))\n  (set! {i} (+ {i} 1)))\n(print {t})\n"
        if r == 5:
            c, inc, dbl = nm("comp"), nm("inc"), nm("dbl")
            return f"(define {c} (lambda (f g) (lambda (x) (f (g x)))))\n(define {inc} (lambda (x) (+ x {val()})))\n(define {dbl} (lambda (x) (* x {rng.randint(2, 4)})))\n(print (({c} {inc} {dbl}) {val()}))\n(print (({c} {dbl} {c}) {inc}))\n".replace(f"(print (({c} {dbl} {c}) {inc}))\n", f"(print (({c} {dbl} {inc}) {val()}))\n")
        if r == 6:
            x, f = nm("x"), nm("shadow")
            return f"(define {x} {val()})\n(define {f} (lambda ({x}) (set! {x} (* {x} 2)) {x}))\n(print ({f} {val()}))\n(print {x})\n(let (({x} 100)) (set! {x} (+ {x} 1)) (print {x}))\n(print {x})\n"
        if r == 7:
            a, b = nm("a"), nm("b")
            return f"(define {a} {val()})\n(define {b} (lambda () (set! {a} (- {a} 1)) {a}))\n(let (({a} {val()}) (z ({b}))) (print (+ {a} z)))\n(print {a})\n"
        if r == 8:
            f = nm("gcd")
            return f"(define {f} (lambda (a b) (if (= b 0) a ({f} b (% a b)))))\n(print ({f} {rng.randint(1, 90)} {rng.randint(1, 90)}))\n"
        if r == 9:
            f = nm("acc")
            return f"(define {f} (lambda (total)\n  (lambda (x)\n    (set! total (+ total x))\n    (and (> total 10) (or (< total 30) (= total 50))))))\n(define a1 ({f} 0))\n(print (a1 {rng.randint(1, 20)}))\n(print (a1 {rng.randint(1, 20)}))\n(print (a1 {rng.randint(1, 20)}))\n".replace("a1", nm("acc"))
        if r == 10:
            return f"(print (if (> {val()} {val()}) {val()} (- {val()} {val()})))\n; {rng.choice(['note', 'again', 'x y z'])}\n(print (begin (+ 1 2) (* {val()} {val()})))\n"
        e = rng.choice([
            "(print (/ 7 (- 3 3)))\n", "(print (+ 1 nothing))\n", "(print (1 2))\n", "(print (+ 1 (< 1 2)))\n",
            "(define z (lambda (a) a))\n(z 1 2)\n", "(if 5 1 2)\n", "(define f2 (lambda (n) (f2 (+ n 1))))\n(f2 0)\n",
            "(define big 1)\n(while #t (set! big (* big 7)))\n", "(set! nope 1)\n", "(lambda ())\n", "(while #t (+ 1 1))\n",
        ])
        return e
    for _ in range(rng.randint(5, 14)):
        s = snip()
        if rng.random() < 0.5: s = "\n" * rng.randint(0, 2) + s
        out.append(s)
    text = "".join(out)
    if rng.random() < 0.08: text += rng.choice(["(print 1\n", ")\n", "(print 12345678901234567)\n"])
    return _c(text)

RESERVED = {"define", "set!", "lambda", "let", "if", "begin", "while", "and", "or", "not", "print",
            "+", "*", "-", "/", "%", "<", ">", "<=", ">=", "="}
LIMIT = 10**15
class ParseErr(Exception):
    def __init__(self, line, msg): self.line, self.msg = line, msg
class RunErr(Exception):
    def __init__(self, line, msg): self.line, self.msg = line, msg

def tokenize(src):
    toks, i, line = [], 0, 1
    while i < len(src):
        ch = src[i]
        if ch == "\n": line += 1; i += 1
        elif ch in " \t\r": i += 1
        elif ch == ";":
            while i < len(src) and src[i] != "\n": i += 1
        elif ch in "()": toks.append((ch, line)); i += 1
        else:
            j = i
            while j < len(src) and src[j] not in " \t\r\n();": j += 1
            toks.append((src[i:j], line)); i = j
    return toks

def parse(src):
    toks = tokenize(src)
    stack, top = [], []
    for t, line in toks:
        if t == "(":
            stack.append((top, line)); top = []
        elif t == ")":
            if not stack: raise ParseErr(line, "unexpected )")
            done = ("list", top, stack[-1][1])
            top = stack.pop()[0]; top.append(done)
        else:
            body = t[1:] if t.startswith("-") else t
            if body != "" and body.isascii() and body.isdigit():
                if len(body) > 15: raise ParseErr(line, "number too large")
                top.append(("int", int(t), line))
            elif t == "#t": top.append(("bool", True, line))
            elif t == "#f": top.append(("bool", False, line))
            else: top.append(("sym", t, line))
    if stack: raise ParseErr(stack[0][1], "unclosed (")
    return top

class Env:
    def __init__(self, parent=None): self.vars, self.parent = {}, parent
    def find(self, n):
        e = self
        while e:
            if n in e.vars: return e
            e = e.parent
        return None

class Closure:
    def __init__(self, params, body, env): self.params, self.body, self.env = params, body, env

def run(src, out):
    prog = parse(src)
    steps, depth = [0], [0]
    def isint(v): return type(v) is int
    def isbool(v): return type(v) is bool
    def show(v):
        if isbool(v): return "#t" if v else "#f"
        if isint(v): return str(v)
        return "<closure>"
    def chk(v, line):
        if abs(v) > LIMIT: raise RunErr(line, "overflow")
        return v
    def names_ok(forms):
        names = []
        for f in forms:
            if f[0] != "sym" or f[1] in RESERVED or f[1] in names: return None
            names.append(f[1])
        return names
    def ev(f, env):
        k = f[0]
        if k == "int" or k == "bool": return f[1]
        if k == "sym":
            if f[1] in RESERVED: raise RunErr(f[2], f"unbound variable {f[1]}")
            e = env.find(f[1])
            if not e: raise RunErr(f[2], f"unbound variable {f[1]}")
            return e.vars[f[1]]
        items, line = f[1], f[2]
        steps[0] += 1
        if steps[0] > 200000: raise RunErr(line, "step limit")
        if not items: raise RunErr(line, "bad syntax")
        h = items[0]
        bad = lambda: RunErr(line, "bad syntax")
        if h[0] == "sym" and h[1] in RESERVED:
            op, a = h[1], items[1:]
            if op == "define":
                if len(a) != 2 or a[0][0] != "sym" or a[0][1] in RESERVED: raise bad()
                env.vars[a[0][1]] = ev(a[1], env); return 0
            if op == "set!":
                if len(a) != 2 or a[0][0] != "sym" or a[0][1] in RESERVED: raise bad()
                v = ev(a[1], env)
                e = env.find(a[0][1])
                if not e: raise RunErr(line, f"unbound variable {a[0][1]}")
                old = e.vars[a[0][1]]; e.vars[a[0][1]] = v; return old
            if op == "lambda":
                if len(a) < 2 or a[0][0] != "list": raise bad()
                ps = names_ok(a[0][1])
                if ps is None: raise bad()
                return Closure(ps, a[1:], env)
            if op == "let":
                if len(a) < 2 or a[0][0] != "list": raise bad()
                names, exprs = [], []
                for b in a[0][1]:
                    if b[0] != "list" or len(b[1]) != 2: raise bad()
                    names.append(b[1][0]); exprs.append(b[1][1])
                if names_ok(names) is None: raise bad()
                vals = [ev(x, env) for x in exprs]
                ne = Env(env)
                for n, v in zip(names, vals): ne.vars[n[1]] = v
                r = 0
                for b in a[1:]: r = ev(b, ne)
                return r
            if op == "if":
                if len(a) not in (2, 3): raise bad()
                c = ev(a[0], env)
                if not isbool(c): raise RunErr(line, "type error")
                if c: return ev(a[1], env)
                return ev(a[2], env) if len(a) == 3 else 0
            if op == "begin":
                r = 0
                for b in a: r = ev(b, env)
                return r
            if op == "while":
                if len(a) < 2: raise bad()
                while True:
                    c = ev(a[0], env)
                    if not isbool(c): raise RunErr(line, "type error")
                    if not c: return 0
                    for b in a[1:]: ev(b, env)
            if op == "and" or op == "or":
                r = op == "and"
                for b in a:
                    v = ev(b, env)
                    if not isbool(v): raise RunErr(line, "type error")
                    r = v
                    if (op == "and") != v: return v
                return r
            if op == "not":
                if len(a) != 1: raise bad()
                v = ev(a[0], env)
                if not isbool(v): raise RunErr(line, "type error")
                return not v
            if op == "print":
                if len(a) != 1: raise bad()
                out.append(show(ev(a[0], env)) + "\n"); return 0
            if op in ("/", "%", "<", ">", "<=", ">=", "=") and len(a) != 2: raise bad()
            if op == "-" and len(a) < 1: raise bad()
            vals = [ev(b, env) for b in a]
            if not all(isint(v) for v in vals): raise RunErr(line, "type error")
            if op == "+":
                t = 0
                for v in vals: t = chk(t + v, line)
                return t
            if op == "*":
                t = 1
                for v in vals: t = chk(t * v, line)
                return t
            if op == "-":
                if len(vals) == 1: return -vals[0]
                t = vals[0]
                for v in vals[1:]: t = chk(t - v, line)
                return t
            if op in ("/", "%"):
                if vals[1] == 0: raise RunErr(line, "division by zero")
                q = abs(vals[0]) // abs(vals[1])
                if (vals[0] < 0) != (vals[1] < 0): q = -q
                return q if op == "/" else vals[0] - q * vals[1]
            x, y = vals
            return {"<": x < y, ">": x > y, "<=": x <= y, ">=": x >= y, "=": x == y}[op]
        fn = ev(h, env)
        if not isinstance(fn, Closure): raise RunErr(line, "not a function")
        args = [ev(b, env) for b in items[1:]]
        if len(args) != len(fn.params): raise RunErr(line, "wrong number of arguments")
        if depth[0] >= 100: raise RunErr(line, "stack overflow")
        ne = Env(fn.env)
        for n, v in zip(fn.params, args): ne.vars[n] = v
        depth[0] += 1
        r = 0
        for b in fn.body: r = ev(b, ne)
        depth[0] -= 1
        return r
    g = Env()
    for f in prog: ev(f, g)

def solve(case):
    res = {}
    def work():
        out = []
        try:
            run(case["stdin"], out)
            res["v"] = ("".join(out), 0)
        except ParseErr as e:
            res["v"] = (f"parse error line {e.line}: {e.msg}\n", 2)
        except RunErr as e:
            res["v"] = ("".join(out) + f"error line {e.line}: {e.msg}\n", 1)
    sys.setrecursionlimit(200000)
    threading.stack_size(512 * 1024 * 1024)
    t = threading.Thread(target=work); t.start(); t.join()
    return res["v"]
