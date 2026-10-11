import sys

def gcd(a, b):
    while b:
        a, b = b, a % b
    return a

data = sys.stdin.read().split()
t = int(data[0])
for i in range(t):
    a, b = int(data[1 + 2 * i]), int(data[2 + 2 * i])
    g = gcd(a, b)
    print(g, a // g * b)
