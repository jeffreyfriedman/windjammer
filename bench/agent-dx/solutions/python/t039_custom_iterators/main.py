import sys
from itertools import count, islice, takewhile


def fib():
    a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b


def squares():
    return (n * n for n in count(1))


def primes():
    found = []
    for n in count(2):
        if all(n % p for p in takewhile(lambda p: p * p <= n, found)):
            found.append(n)
            yield n


def evens():
    return count(0, 2)


def triangular():
    t = 0
    for n in count(1):
        t += n
        yield t


GEN = {"fib": fib, "squares": squares, "primes": primes, "evens": evens, "triangular": triangular}
for line in sys.stdin.read().split("\n"):
    p = line.split()
    if len(p) != 3:
        continue
    g, mode, n = GEN[p[0]](), p[1], int(p[2])
    if mode == "take":
        print(" ".join(map(str, islice(g, n))))
    elif mode == "below":
        print(" ".join(map(str, takewhile(lambda v: v < n, g))))
    else:
        print(sum(islice(g, n)))
