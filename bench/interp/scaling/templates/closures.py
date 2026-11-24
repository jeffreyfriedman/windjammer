import sys, time
t0 = time.perf_counter_ns()
v = list(range(@len@))
total = 0
for r in range(@rounds@):
    s = sum(x // 3 for x in (x * r for x in v) if x % 3 == 0)
    total += s % 100000
print("STEADY_NS", time.perf_counter_ns() - t0, file=sys.stderr)
print(total)
