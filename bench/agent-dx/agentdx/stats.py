"""Bootstrap statistics. Tasks are the sampling unit (trials within a task are correlated), so the
bootstrap resamples tasks, then trials within each resampled task."""
import random


def mean(xs):
    xs = [x for x in xs if x is not None]
    return sum(xs) / len(xs) if xs else None


def _cluster_mean(clusters, rng):
    pick = [clusters[rng.randrange(len(clusters))] for _ in clusters]
    vals = []
    for c in pick:
        vals += [c[rng.randrange(len(c))] for _ in c]
    return sum(vals) / len(vals)


def bootstrap_ci(clusters, n_boot=2000, alpha=0.05, seed=0):
    """clusters: list (per task) of lists of values. -> (mean, lo, hi) or None when empty."""
    clusters = [[x for x in c if x is not None] for c in clusters]
    clusters = [c for c in clusters if c]
    if not clusters:
        return None
    rng = random.Random(seed)
    flat = [x for c in clusters for x in c]
    ms = sorted(_cluster_mean(clusters, rng) for _ in range(n_boot))
    return (sum(flat) / len(flat), ms[int(alpha / 2 * n_boot)], ms[int((1 - alpha / 2) * n_boot) - 1])


def paired_diff_ci(a_clusters, b_clusters, n_boot=2000, alpha=0.05, seed=0):
    """a_clusters/b_clusters: dict task -> list of values. CI for mean(A) - mean(B) over the shared tasks."""
    tasks = sorted(set(a_clusters) & set(b_clusters))
    tasks = [t for t in tasks if any(x is not None for x in a_clusters[t]) and any(x is not None for x in b_clusters[t])]
    if not tasks:
        return None
    a = {t: [x for x in a_clusters[t] if x is not None] for t in tasks}
    b = {t: [x for x in b_clusters[t] if x is not None] for t in tasks}
    rng = random.Random(seed)

    def one():
        pick = [tasks[rng.randrange(len(tasks))] for _ in tasks]
        da = [a[t][rng.randrange(len(a[t]))] for t in pick for _ in a[t]]
        db = [b[t][rng.randrange(len(b[t]))] for t in pick for _ in b[t]]
        return sum(da) / len(da) - sum(db) / len(db)
    ds = sorted(one() for _ in range(n_boot))
    pt = mean([x for t in tasks for x in a[t]]) - mean([x for t in tasks for x in b[t]])
    return (pt, ds[int(alpha / 2 * n_boot)], ds[int((1 - alpha / 2) * n_boot) - 1])
