from collections import defaultdict

POOL = ["listen", "silent", "enlist", "google", "gogole", "cat", "act", "tac", "dog", "god", "a", "b", "abc", "cab", "bca", "Listen", "TAC"]


def _c(words_per_line):
    return {"stdin": "".join(" ".join(w) + "\n" for w in words_per_line)}


SAMPLES = [_c([["listen", "silent", "enlist", "google", "gogole"]]), _c([["abc", "def"]])]
EDGE = [_c([[]]), _c([["Listen", "SILENT", "listen"]]), _c([["a", "A"]]), _c([["ab", "ba"], ["cd", "dc"], ["x"]])]


def rand(rng):
    return _c([[rng.choice(POOL) for _ in range(rng.randint(0, 6))] for _ in range(rng.randint(1, 4))])


def solve(case):
    g = defaultdict(set)
    for w in case["stdin"].split():
        w = w.lower()
        g[tuple(sorted(w))].add(w)
    lines = sorted(" ".join(sorted(s)) for s in g.values() if len(s) >= 2)
    return "".join(l + "\n" for l in lines) if lines else "none\n"
