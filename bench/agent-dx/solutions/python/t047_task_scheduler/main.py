import sys

d = [l.split() for l in sys.stdin.read().split("\n") if l.strip()]
w, n = int(d[0][0]), int(d[0][1])
tasks = {}
for row in d[1:1 + n]:
    tasks[int(row[0])] = (int(row[1]), [int(x) for x in row[2:]])
start = {}
end = {}
running = []  # (end, id)
t = 0
pending = sorted(tasks)
while pending or running:
    done_now = [r for r in running if r[0] <= t]
    running = [r for r in running if r[0] > t]
    free = w - len(running)
    for tid in list(pending):
        if free == 0:
            break
        dur, deps = tasks[tid]
        if all(x in end and end[x] <= t for x in deps):
            start[tid] = t
            end[tid] = t + dur
            running.append((t + dur, tid))
            pending.remove(tid)
            free -= 1
    if running:
        t = min(r[0] for r in running)
    else:
        break
for tid in sorted(tasks):
    print(tid, start[tid], end[tid])
print("makespan", max(end.values()) if end else 0)
