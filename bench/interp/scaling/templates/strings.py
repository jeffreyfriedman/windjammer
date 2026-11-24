import sys, time
t0 = time.perf_counter_ns()
s = ""
total = 0
for i in range(@iters@):
    s += f"{i % 10},"
    if len(s) > 1000:
        total += len(s)
        s = ""
print("STEADY_NS", time.perf_counter_ns() - t0, file=sys.stderr)
print(total, len(s))
