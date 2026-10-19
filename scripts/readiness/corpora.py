"""Discovery of projects (R1/R3/R4/R6) and single-file program corpora (R2/R3/R5), plus config loading."""
import glob, os, re

try:
    import tomllib
except ImportError:  # python < 3.11: the config is a flat subset we can read by hand
    tomllib = None


class Project:
    def __init__(self, name, path, private=False, index=0):
        self.name, self.path, self.private = name, os.path.abspath(path), private
        self.label = f"private-{index}" if private else name

    def wj_files(self):
        out = []
        for d, dirs, files in os.walk(self.path):
            dirs[:] = [x for x in dirs if x not in ("build", "target", ".git")]
            out += [os.path.join(d, f) for f in files if f.endswith(".wj")]
        return sorted(out)


def _mini_toml(text):
    data, cur = {}, None
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        if line == "[[project]]":
            cur = {}
            data.setdefault("project", []).append(cur)
        elif "=" in line:
            k, v = [s.strip() for s in line.split("=", 1)]
            v = v.strip('"') if v.startswith('"') else (v == "true" if v in ("true", "false") else v)
            (cur if cur is not None else data)[k] = v
    return data


def load_config(path):
    """Config lives outside the repo (default ~/.config/wj-readiness.toml, or $WJ_READINESS_CONFIG):
    `ecosystem = "<dir>"` and `[[project]] name/path/private`. $WJ_READINESS_PROJECTS="name=path;..."
    adds projects (treated as private, so their names never reach committed output)."""
    cfg = {"ecosystem": None, "project": []}
    path = path or os.environ.get("WJ_READINESS_CONFIG") or os.path.expanduser("~/.config/wj-readiness.toml")
    if os.path.exists(path):
        text = open(path).read()
        data = tomllib.loads(text) if tomllib else _mini_toml(text)
        cfg["ecosystem"] = data.get("ecosystem")
        cfg["project"] = list(data.get("project", []))
    for item in filter(None, os.environ.get("WJ_READINESS_PROJECTS", "").split(";")):
        n, _, p = item.partition("=")
        cfg["project"].append({"name": n, "path": p, "private": True})
    return cfg


def has_tests(path):
    for d, dirs, files in os.walk(path):
        dirs[:] = [x for x in dirs if x not in ("build", "target", ".git")]
        if any(f.endswith(("_test.wj", "_tests.wj")) or f.startswith("test_") and f.endswith(".wj") for f in files):
            return True
    return False


def build_projects(root, cfg, quick, ecosystem=None):
    eco = ecosystem or cfg.get("ecosystem") or os.path.join(root, "..", "windjammer-ecosystem")
    eco = os.path.abspath(os.path.expanduser(eco))
    projects = []
    for kind in ("packages", "apps"):
        found = [d for d in sorted(glob.glob(os.path.join(eco, kind, "*"))) if os.path.isdir(d) and has_tests(d)]
        if quick:
            found = found[:2]
        projects += [Project(os.path.basename(d), d) for d in found]
    n = 0
    for p in cfg.get("project", []):
        if not os.path.isdir(p.get("path", "")):
            continue
        priv = bool(p.get("private", True))
        n += priv
        projects.append(Project(p["name"], p["path"], priv, n))
    return projects


def _sample(items, n):
    if n is None or len(items) <= n:
        return items
    step = len(items) / n
    return [items[int(i * step)] for i in range(n)]


class Program:
    """A single-file program with an oracle parsed from its header comments."""
    def __init__(self, corpus, path, src):
        self.corpus, self.path, self.src = corpus, path, src
        self.name = f"{corpus}/{os.path.basename(path)}"
        h = {}
        for line in src.splitlines():
            m = re.match(r"// ([a-z-]+):\s?(.*)$", line)
            if m:
                h.setdefault(m.group(1), []).append(m.group(2))
        self.header = h
        self.expect = h.get("expect")
        self.mode = (h.get("mode") or ["run"])[0].strip()
        self.exit = int((h.get("exit") or h.get("expect-exit") or ["0"])[0]) if (h.get("exit") or h.get("expect-exit")) else None
        self.xfail = "xfail" in h
        self.interp_only = "interp-only" in h
        self.seeds = [int(s) for s in " ".join(h.get("seeds", [])).split()]

    def runnable(self):
        """Comparable interpreted-vs-compiled: plain run/panic programs without manifest grants."""
        return self.mode in ("run", "panic") and "manifest" not in self.header and not self.xfail

    def expected_stdout(self):
        return ("\n".join(self.expect) + "\n") if self.expect and self.mode == "run" else None


def load_programs(root, quick):
    n = 8 if quick else None
    corp = {}
    for name, pattern in (("verified", "examples/verified/*.wj"), ("conformance", "tests/conformance_programs/*.wj"),
                          ("spec", "tests/spec/*/*.wj")):
        files = sorted(glob.glob(os.path.join(root, pattern)))
        progs = [Program(name, f, open(f, errors="replace").read()) for f in files]
        corp[name] = _sample([p for p in progs if p.runnable()], n)
    return corp


def load_templates(root):
    """Generated-differential templates (`__SEED__`) that ship an independent python reference model."""
    out = []
    for f in sorted(glob.glob(os.path.join(root, "tests/interpreter_corpus/*.wj"))):
        ref = f[:-3] + ".ref.py"
        if os.path.exists(ref):
            p = Program("generated", f, open(f).read())
            p.ref = ref
            out.append(p)
    return out
