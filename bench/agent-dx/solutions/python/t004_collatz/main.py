import sys

data = sys.stdin.read().split()
t = int(data[0])
for s in data[1:1 + t]:
    n = int(s)
    steps = 0
    peak = n
    while n != 1:
        n = n // 2 if n % 2 == 0 else 3 * n + 1
        steps += 1
        if n > peak:
            peak = n
    print(steps, peak)
