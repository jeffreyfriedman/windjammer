import sys, time
t0 = time.perf_counter_ns()
m = {}
for i in range(@iters@):
    k = (i * 7919) % @keys@
    m[k] = m.get(k, 0) + 1
best = 0
for j in range(@keys@):
    v = m.get(j)
    if v is not None and v > best:
        best = v
print("STEADY_NS", time.perf_counter_ns() - t0, file=sys.stderr)
print(len(m), best)
