import sys


def fib(n, m):
    if n == 0:
        return 0 % m
    a, b = 0, 1  # F(k), F(k+1)
    for bit in bin(n)[2:]:
        c = a * ((2 * b - a) % m) % m
        d = (a * a + b * b) % m
        if bit == "1":
            a, b = d, (c + d) % m
        else:
            a, b = c, d
    return a % m


data = sys.stdin.read().split()
t = int(data[0])
for i in range(t):
    print(fib(int(data[1 + 2 * i]), int(data[2 + 2 * i])))
