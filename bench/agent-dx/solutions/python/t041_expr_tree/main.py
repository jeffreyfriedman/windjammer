import sys
from dataclasses import dataclass


@dataclass
class Lit:
    v: int


@dataclass
class Neg:
    a: object


@dataclass
class Bin:
    op: str
    a: object
    b: object


def parse(toks, i):
    t = toks[i]
    if t != "(":
        return Lit(int(t)), i + 1
    op = toks[i + 1]
    a, i = parse(toks, i + 2)
    if op == "neg":
        return Neg(a), i + 1
    b, i = parse(toks, i)
    return Bin(op, a, b), i + 1


def evaluate(n):
    if isinstance(n, Lit):
        return n.v
    if isinstance(n, Neg):
        return -evaluate(n.a)
    x, y = evaluate(n.a), evaluate(n.b)
    return x + y if n.op == "+" else x - y if n.op == "-" else x * y


def depth(n):
    if isinstance(n, Lit):
        return 1
    if isinstance(n, Neg):
        return 1 + depth(n.a)
    return 1 + max(depth(n.a), depth(n.b))


def nodes(n):
    if isinstance(n, Lit):
        return 1
    if isinstance(n, Neg):
        return 1 + nodes(n.a)
    return 1 + nodes(n.a) + nodes(n.b)


def render(n):
    if isinstance(n, Lit):
        return str(n.v)
    if isinstance(n, Neg):
        return "-(%s)" % render(n.a)
    return "(%s %s %s)" % (render(n.a), n.op, render(n.b))


sys.setrecursionlimit(100000)
for line in sys.stdin.read().split("\n"):
    line = line.strip()
    if not line:
        continue
    toks = line.replace("(", "( ").replace(")", " )").split()
    tree, _ = parse(toks, 0)
    print(evaluate(tree), depth(tree), nodes(tree), render(tree))
