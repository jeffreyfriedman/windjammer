import sys


class Memo:
    def __init__(self, fn):
        self.fn = fn
        self.cache = {}
        self.hits = 0
        self.misses = 0

    def __call__(self, arg):
        if arg in self.cache:
            self.hits += 1
            return self.cache[arg], "hit"
        self.misses += 1
        v = self.cache[arg] = self.fn(arg)
        return v, "miss"


def fib(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, (a + b) % 1000000007
    return a


def collatz(n):
    steps = 0
    while n != 1:
        n = n // 2 if n % 2 == 0 else 3 * n + 1
        steps += 1
    return steps


def tri(n):
    return n * (n + 1) // 2


memos = {"fib": Memo(fib), "collatz": Memo(collatz), "tri": Memo(tri)}
hits = misses = 0
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if not p:
        continue
    if p[0] not in memos or len(p) < 2:
        print("unknown function")
        continue
    v, st = memos[p[0]](int(p[1]))
    print(v, st)
    if st == "hit":
        hits += 1
    else:
        misses += 1
print("hits=%d misses=%d" % (hits, misses))
