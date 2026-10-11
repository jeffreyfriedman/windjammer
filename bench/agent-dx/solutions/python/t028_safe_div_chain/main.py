import sys

lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
MIN = -(1 << 63)
for line in lines:
    nums = [int(x) for x in line.split(" / ")]
    v = nums[0]
    err = None
    for k, d in enumerate(nums[1:], 1):
        if d == 0:
            err = "error: division by zero at step %d" % k
            break
        if v == MIN and d == -1:
            err = "error: overflow at step %d" % k
            break
        q = abs(v) // abs(d)
        v = q if (v < 0) == (d < 0) else -q
    print(err if err else v)
