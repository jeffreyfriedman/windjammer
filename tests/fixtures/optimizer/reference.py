#!/usr/bin/env python3
"""Independent reference model for the optimizer fixtures.

Writes `<name>.expected` (one line per seed 1..5) next to each fixture. The
models are straight Python re-implementations of the intended semantics; they
share no code with the Rust fixtures or the optimizer.
"""
import os
import sys

M = (1 << 64) - 1
SEEDS = [1, 2, 3, 4, 5]
FNV0 = 0xcbf29ce484222325


class Rng:
    def __init__(self, seed):
        self.s = ((seed * 0x9E3779B97F4A7C15) & M) | 1

    def next(self):
        x = self.s
        x ^= x >> 12
        x ^= (x << 25) & M
        x ^= x >> 27
        self.s = x
        return (x * 0x2545F4914F6CDD1D) & M


def fold(h, v):
    return ((h ^ (v & M)) * 0x100000001B3) & M


def p01(seed):
    r = Rng(seed)
    n = 1000 + r.next() % 500
    v = [r.next() % 1000 for _ in range(n)]
    h = FNV0
    for i in range(n):
        h = fold(h, v[i] * i)
    return str(h)


def p02(seed):
    r = Rng(seed)
    n = 24
    a, b = [], []
    for _ in range(n * n):
        a.append(r.next() % 100)
        b.append(r.next() % 100)
    h = FNV0
    for i in range(n):
        for j in range(n):
            pass
    c = [0] * (n * n)
    for i in range(n):
        for j in range(n):
            c[i * n + j] = sum(a[i * n + k] * b[k * n + j] for k in range(n))
    for i in range(n * n):
        h = fold(h, c[i])
    return str(h)


def p03(seed):
    n = 20000 + seed * 1000
    f = [True] * (n + 1)
    f[0] = f[1] = False
    i = 2
    while i * i <= n:
        if f[i]:
            for j in range(i * i, n + 1, i):
                f[j] = False
        i += 1
    h, c = FNV0, 0
    for i in range(n + 1):
        if f[i]:
            c += 1
            h = fold(h, i)
    return f"{c} {h}"


def p04(seed):
    r = Rng(seed)
    words = [r.next() % 1000 for _ in range(200)]
    s = "".join(f"{w}," for w in words)
    h = FNV0
    for b in s.encode():
        h = fold(h, b)
    return f"{len(s)} {h}"


def p05(seed):
    out = "a"
    for _ in range(8):
        out = out + str(len(out))
    t = ""
    for i in range(30):
        t = t + f"{i * seed}-{'true' if i % 3 == 0 else 'false'};"
    u = "x"
    u = u + u
    u = u + u
    return f"{out} {t} {u}"


def p06(seed):
    r = Rng(seed)
    base = [1, 2, 3]
    v = [r.next() % 10 for _ in range(50)]
    snap = list(v)
    v.append(99)
    v[0] = 7
    h = FNV0
    for x in snap:
        h = fold(h, x)
    for x in v:
        h = fold(h, x * 3)
    for x in base:
        h = fold(h, x)
    return f"{h} abc {len(snap) + len(v)}"


def p07(seed):
    r = Rng(seed)
    a = r.next() % 100
    b = r.next() % 1000
    c = 3.5
    res = a + b + int(c * 2.0)
    return f"{res} true qq"


def p08(seed):
    r = Rng(seed)
    k = r.next() % 5000 - 2500
    x = r.next() % 256
    def wrap(v, bits, signed):
        v &= (1 << bits) - 1
        if signed and v >= 1 << (bits - 1):
            v -= 1 << bits
        return v
    a = 18
    b = wrap(k, 32, True)
    c = wrap(k, 8, False)
    d = wrap(k, 32, True)
    e = wrap(k, 16, True)
    y = x + x
    return f"{a} {b} {c} {d} {e} {y}"


def p09(seed):
    r = Rng(seed)
    v = []
    i = 0
    while True:
        x = r.next() % 1000
        v.append(x)
        if x < 10 and i > 3:
            break
        i += 1
    h = FNV0
    for x in v:
        h = fold(h, x)
    return f"{len(v)} {h}"


def p10(seed):
    a = [i * i for i in range(3 + seed, 40)]
    h = FNV0
    for x in a:
        h = fold(h, x)
    return f"{len(a)} 0 {h}"


def p11(seed):
    r = Rng(seed)
    a = r.next() % 256
    b = r.next() % 100000
    h = FNV0
    for byte in bytes([a]) + b.to_bytes(4, "little"):
        h = fold(h, byte)
    return f"{h} 0"


def p12(seed):
    best, arg = 0, 0
    for n in range(1, 3000 + seed * 100):
        s, m = 0, n
        while m != 1:
            m = m // 2 if m % 2 == 0 else 3 * m + 1
            s += 1
        if s > best:
            best, arg = s, n
    return f"{arg} {best}"


def p13(seed):
    r = Rng(seed)
    v = sorted(r.next() % 10000 for _ in range(300))
    h = FNV0
    for i, x in enumerate(v):
        h = fold(h, x * (i + 1))
    return str(h)


def p14(seed):
    r = Rng(seed)
    counts = {}
    for _ in range(5000):
        k = r.next() % 64
        counts[k] = counts.get(k, 0) + 1
    h = FNV0
    for k in sorted(counts):
        h = fold(h, k)
        h = fold(h, counts[k])
    return f"{len(counts)} {h}"


def p15(seed):
    return f'5 5 ["{seed}", "{seed}", "hi", "hi"]'


def fib(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a


def p19(seed):
    return f"12 {seed + seed * 7 + 1}"


def p20(seed):
    r = Rng(seed)
    n = r.next() % 1000
    d = f"kernel:{n}:kernel"
    return f"{d} kernel 2 kernel {d}"


def p16(seed):
    return str(fib(15 + seed))


def p17(seed):
    r = Rng(seed)
    g = [[r.next() % 100 for _ in range(30)] for _ in range(20)]
    h = FNV0
    for ri in range(20):
        for c in range(30):
            h = fold(h, g[ri][c] * (ri * 31 + c))
    return str(h)


def p18(seed):
    r = Rng(seed)
    bs = [r.next() % 256 for _ in range(500)]
    h = FNV0
    for b in bs:
        h = fold(h, b)
    return f"{h} {sum(bs)}"


MODELS = {
    "p01_fill_sum": p01, "p02_matmul": p02, "p03_sieve": p03, "p04_string_build": p04,
    "p05_string_selfref": p05, "p06_clone_temp": p06, "p07_copy_clone": p07, "p08_casts": p08,
    "p09_early_break": p09, "p10_nonzero_start": p10, "p11_repr_to_bytes": p11, "p12_collatz": p12,
    "p13_insertion_sort": p13, "p14_hashmap_counts": p14, "p15_shadow_traps": p15,
    "p16_inline_recursion": p16, "p17_nested_capacity": p17, "p18_byte_hash": p18, "p19_repr_layout": p19, "p20_macro_clone": p20,
}

if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    for name, f in MODELS.items():
        with open(os.path.join(here, name + ".expected"), "w") as out:
            for s in SEEDS:
                out.write(f(s) + "\n")
    print("wrote", len(MODELS), "expected files", file=sys.stderr)
