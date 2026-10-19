"""Process runner, resumable result cache, and the context every criterion shares."""
import hashlib, json, os, re, resource, subprocess, time

QUICK, FULL = "quick", "full"


class Res:
    """One finished command."""
    def __init__(self, rc, out, err, secs, cpu=0.0, timed_out=False):
        self.rc, self.out, self.err, self.secs, self.cpu, self.timed_out = rc, out, err, secs, cpu, timed_out

    def to_json(self):
        return dict(rc=self.rc, out=self.out, err=self.err, secs=self.secs, cpu=self.cpu, timed_out=self.timed_out)

    @staticmethod
    def from_json(d):
        return Res(d["rc"], d["out"], d["err"], d["secs"], d.get("cpu", 0.0), d.get("timed_out", False))


def run(cmd, timeout=300, cwd=None, env=None, stdin=None):
    """Run serially, bounded by `timeout`; never raises. Child CPU time comes from rusage deltas."""
    e = dict(os.environ, NO_COLOR="1")
    e.update(env or {})
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.monotonic()
    try:
        p = subprocess.run(cmd, cwd=cwd, env=e, capture_output=True, timeout=timeout, input=stdin)
        rc, out, err, to = p.returncode, p.stdout, p.stderr, False
    except subprocess.TimeoutExpired as x:
        rc, out, err, to = -9, x.stdout or b"", x.stderr or b"", True
    except OSError as x:
        rc, out, err, to = 127, b"", str(x).encode(), False
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    return Res(rc, out.decode("utf-8", "replace"), err.decode("utf-8", "replace"), time.monotonic() - t0, cpu, to)


def sha(*parts):
    h = hashlib.sha256()
    for p in parts:
        h.update(p if isinstance(p, bytes) else str(p).encode())
        h.update(b"\0")
    return h.hexdigest()


def file_hash(path):
    try:
        with open(path, "rb") as f:
            return hashlib.sha256(f.read()).hexdigest()
    except OSError:
        return "missing"


def tree_hash(path, exts=(".wj", ".toml")):
    """Content hash of a directory's sources (the 'program hash' of a project)."""
    h = hashlib.sha256()
    for d, dirs, files in os.walk(path):
        dirs[:] = sorted(x for x in dirs if x not in ("build", "target", ".git"))
        for f in sorted(files):
            if f.endswith(exts):
                p = os.path.join(d, f)
                h.update(os.path.relpath(p, path).encode())
                h.update(file_hash(p).encode())
    return h.hexdigest()


class Cache:
    """Resumable results keyed by sha(commit, wj binary hash, kind, program hash). An interrupted
    --full run resumes from the entries already written; a new commit or binary invalidates them."""
    def __init__(self, directory, enabled=True):
        self.dir, self.enabled = directory, enabled
        if enabled:
            os.makedirs(directory, exist_ok=True)
        self.hits = self.misses = 0

    def _path(self, key):
        return os.path.join(self.dir, key + ".json")

    def get(self, key):
        if not self.enabled:
            return None
        try:
            with open(self._path(key)) as f:
                self.hits += 1
                return json.load(f)
        except (OSError, ValueError):
            self.misses += 1
            return None

    def put(self, key, value):
        if self.enabled:
            tmp = self._path(key) + ".tmp"
            with open(tmp, "w") as f:
                json.dump(value, f)
            os.replace(tmp, self._path(key))


class Ctx:
    """Everything a criterion needs: the wj binary, repo root, mode, cache, project list, runner."""
    def __init__(self, wj, root, mode, cache, projects, commit="unknown", timeouts=None, runner=run, log=None):
        self.wj, self.root, self.mode, self.cache, self.projects = wj, root, mode, cache, projects
        self.commit, self.runner, self.log = commit, runner, log or (lambda m: None)
        self.timeouts = dict(interp=120, compiled=900, check=60, build=300)
        self.timeouts.update(timeouts or {})
        self.wj_hash = file_hash(wj)
        self.cold = False
        self.quick = mode == QUICK

    def key(self, kind, program_hash):
        return sha(self.commit, self.wj_hash, kind, program_hash)

    def cached_run(self, kind, program_hash, cmd, timeout, keep_timeouts=False, **kw):
        """Run `cmd` once per (commit, binary, kind, program hash); later calls replay the stored result."""
        k = self.key(kind, program_hash)
        hit = self.cache.get(k)
        if hit is not None:
            return Res.from_json(hit)
        self.log("run: " + kind)
        r = self.runner(cmd, timeout=timeout, **kw)
        # timeouts are environmental (machine load): never cached, so a quieter rerun can succeed
        if keep_timeouts or not r.timed_out:
            self.cache.put(k, r.to_json())
        return r

    def target_root(self, scratch=None):
        """Private cargo target root for compiled runs (WJ_CARGO_TARGET_ROOT): isolates them from other
        builds on the machine; `scratch` names a throwaway root, so the run starts with a cold cache."""
        if scratch:
            return os.path.join(self.cache.dir, "cold-target", scratch)
        return os.path.join(self.cache.dir, "cargo-target")

    def is_private(self, name):
        return any(p.private and p.name == name for p in self.projects)


def alias(name, private, index):
    return f"private-{index}" if private else name


def scrub(text, ctx_paths, limit=160):
    """One-line, path-free reason: configured project paths and the home directory are removed."""
    t = text.strip().splitlines()[0] if text.strip() else ""
    for p, repl in ctx_paths:
        if p:
            t = t.replace(p, repl)
    t = re.sub(r"/(?:Users|home)/[^\s:)\"']+", "<path>", t)
    t = re.sub(r"\x1b\[[0-9;]*m", "", t)
    return t[:limit]


def pct(num, den):
    return round(100.0 * num / den, 1) if den else None
