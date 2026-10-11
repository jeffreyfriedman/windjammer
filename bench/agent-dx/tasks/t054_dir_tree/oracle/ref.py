import os


def _c(files):
    return {"stdin": "", "args": [], "files": files}


SAMPLES = [_c({"a.txt": "hello", "src/main.rs": "fn main() {}\n"}), _c({"x": ""})]
EDGE = [_c({}), _c({"b/c/d/e.txt": "deep", "b/a": "1", "B": "upper", "a/z": "zz", "a b": "space"})]


def rand(rng):
    files = {}
    for _ in range(rng.randint(1, 10)):
        depth = rng.randint(0, 3)
        p = "/".join(rng.choice(["a", "b", "cc", "D"]) for _ in range(depth)) + ("/" if depth else "") + rng.choice(["f1", "f2.txt", "g", "Z"])
        if any(q != p and (q.startswith(p + "/") or p.startswith(q + "/")) for q in files):
            continue
        files[p] = "x" * rng.randint(0, 40)
    return _c(files)


def solve(case):
    tree = {}
    for p, c in case["files"].items():
        node = tree
        parts = p.split("/")
        for d in parts[:-1]:
            node = node.setdefault(d, {})
        node[parts[-1]] = len(c.encode())
    out, counts = [], [0, 0]

    def walk(n, depth):
        for k in sorted(n, key=lambda s: s.encode()):
            if isinstance(n[k], dict):
                counts[0] += 1
                out.append("  " * depth + k + "/")
                walk(n[k], depth + 1)
            else:
                counts[1] += 1
                out.append("  " * depth + f"{k} ({n[k]} bytes)")
    walk(tree, 0)
    out.append(f"{counts[0]} directories, {counts[1]} files")
    return "\n".join(out) + "\n"
