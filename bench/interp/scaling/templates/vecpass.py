import sys, time
t0 = time.perf_counter_ns()
v = []
for k in range(@len@):
    v.append(1.5)
def get(v, i):
    return v[i]
s = 0.0
for c in range(@calls@):
    s += get(v, c % @len@)
print("STEADY_NS", time.perf_counter_ns() - t0, file=sys.stderr)
print(int(s * 2.0))
