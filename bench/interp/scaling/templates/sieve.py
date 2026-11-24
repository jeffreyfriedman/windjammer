import sys, time
t0 = time.perf_counter_ns()
n = @n@
flags = [True] * n
count = 0
i = 2
while i < n:
    if flags[i]:
        count += 1
        j = i * i
        while j < n:
            flags[j] = False
            j += i
    i += 1
print("STEADY_NS", time.perf_counter_ns() - t0, file=sys.stderr)
print(count)
