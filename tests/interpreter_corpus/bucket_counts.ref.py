import sys
names = ["alpha", "beta", "delta", "eps", "gamma"]
state = int(sys.argv[1]); counts = [0] * 5
for _ in range(300):
    state = (state * 1103515245 + 12345) % 2147483648
    counts[(state // 65536) % 5] += 1
for n, c in zip(names, counts): print(n, c)
