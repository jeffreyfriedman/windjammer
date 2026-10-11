def _c(w, tasks):
    return {"stdin": f"{w} {len(tasks)}\n" + "".join(" ".join(map(str, [i, d] + deps)) + "\n" for i, d, deps in tasks)}


SAMPLES = [_c(2, [(1, 3, []), (2, 2, []), (3, 4, [1, 2]), (4, 1, [1])]), _c(1, [(5, 2, []), (3, 1, [])])]
EDGE = [_c(3, []), _c(2, [(1, 1, []), (2, 1, [1]), (3, 1, [2]), (4, 1, [3])]), _c(8, [(i, 1, []) for i in range(1, 7)]), _c(2, [(10, 5, []), (20, 1, []), (30, 1, [20]), (40, 1, [30]), (50, 1, [40, 10])])]


def rand(rng):
    n = rng.randint(1, 12)
    ids = rng.sample(range(1, 40), n)
    order = list(ids)
    tasks = []
    for idx, i in enumerate(order):
        deps = rng.sample(order[:idx], min(idx, rng.randint(0, 3)))
        tasks.append((i, rng.randint(1, 6), deps))
    rng.shuffle(tasks)
    return _c(rng.randint(1, 4), tasks)


def solve(case):
    ls = case["stdin"].split("\n")[:-1]
    w, n = map(int, ls[0].split())
    T = {}
    for l in ls[1:1 + n]:
        p = list(map(int, l.split()))
        T[p[0]] = (p[1], p[2:])
    start, end, running, t = {}, {}, [], 0
    while len(end) < n:
        running = [r for r in running if r[0] > t] if False else running
        # finish tasks
        done_now = [r for r in running if r[0] <= t]
        running = [r for r in running if r[0] > t]
        for e, i in done_now:
            end[i] = e
        ready = sorted(i for i in T if i not in start and all(d in end and end[d] <= t for d in T[i][1]))
        for i in ready:
            if len(running) < w:
                start[i] = t
                running.append((t + T[i][0], i))
        if len(end) >= n:
            break
        if running:
            t = min(r[0] for r in running)
        else:
            break
    out = [f"{i} {start[i]} {end[i]}" for i in sorted(T)]
    out.append(f"makespan {max(end.values()) if end else 0}")
    return "\n".join(out) + "\n"
