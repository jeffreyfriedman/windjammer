import sys

n, a, b = map(int, sys.stdin.read().split()[:3])
out = []
for i in range(1, n + 1):
    if i % a == 0 and i % b == 0:
        out.append("FizzBuzz")
    elif i % a == 0:
        out.append("Fizz")
    elif i % b == 0:
        out.append("Buzz")
    else:
        out.append(str(i))
if out:
    print("\n".join(out))
