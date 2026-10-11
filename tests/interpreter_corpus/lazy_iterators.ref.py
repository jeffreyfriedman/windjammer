import sys, itertools
s = int(sys.argv[1])
odds = list(itertools.islice((x for x in (v * v for v in itertools.count(s)) if x % 2 == 1), 4))
print(odds)
print([s + (i % 3) for i in range(8)])
under = []
x = 1
while x * s < 10 * s:
    under.append(x * s); x += 1
print(under)
x = 0
while x * s < 3 * s:
    print("make %d" % x); print("use %d" % (x * s)); x += 1
print("make %d" % x)
f = next(v for v in itertools.count(s) if v % 7 == 0)
print("Some(%d)" % f)
print("Some(%d)" % next(x for x in itertools.count(0) if x * x > s * 100))
print("Some(%d) Some(%d)" % (s, s + 3))
print(list(reversed(list(range(1, 4)) + list(range(10, 12)))))
n = 0; i = 0
while True:
    n += i * s
    if n > 40:
        print("stop at %d" % i); break
    i += 1
ev = [x for x in range(20) if x % 2 == 0][s % 3:][::2]
print(ev)
