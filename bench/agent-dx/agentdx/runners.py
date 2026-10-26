"""Per-language build + run. One class per language, uniform interface.

build(workdir)  -> BuildResult   (workdir holds main.<ext>; artifacts go in workdir/.build)
run(build, case, workdir) -> RunResult   (case = {stdin, args, files})
"""
import json
import os
import shutil
import subprocess
import tempfile
import threading
import time

from . import diagnostics as dg
from .toolchains import wj_bin


def _scale():
    return float(os.environ.get("AGENTDX_TIMEOUT_SCALE", "1"))


class BuildResult:
    def __init__(self, ok, diagnostics=None, raw="", seconds=0.0, prog=None):
        self.ok, self.diagnostics, self.raw, self.seconds, self.prog = ok, diagnostics or [], raw, seconds, prog

    def to_dict(self):
        return {"ok": self.ok, "diagnostics": self.diagnostics, "seconds": round(self.seconds, 3),
                "raw": self.raw[-4000:]}


class RunResult:
    def __init__(self, stdout="", stderr="", code=0, timed_out=False, seconds=0.0):
        self.stdout, self.stderr, self.code, self.timed_out, self.seconds = stdout, stderr, code, timed_out, seconds


def _exec(cmd, cwd, stdin_text="", timeout=60, env=None):
    t0 = time.time()
    e = dict(os.environ)
    if env:
        e.update(env)
    try:
        p = subprocess.run(cmd, cwd=cwd, input=stdin_text, capture_output=True, text=True,
                           timeout=timeout * _scale(), env=e, errors="replace")
        return p.returncode, p.stdout, p.stderr, False, time.time() - t0
    except subprocess.TimeoutExpired as ex:
        out = ex.stdout.decode(errors="replace") if isinstance(ex.stdout, bytes) else (ex.stdout or "")
        err = ex.stderr.decode(errors="replace") if isinstance(ex.stderr, bytes) else (ex.stderr or "")
        return -9, out, err, True, time.time() - t0


class Runner:
    lang = ""
    src = "main.txt"
    build_timeout = 120
    run_timeout = 10
    arg_separator = []  # tokens placed between the program and its arguments

    def build_dir(self, wd):
        d = os.path.join(wd, ".build")
        os.makedirs(d, exist_ok=True)
        return d

    def build(self, wd):
        raise NotImplementedError

    def command(self, build, wd):
        raise NotImplementedError

    def run(self, build, case, wd=None):
        with tempfile.TemporaryDirectory(prefix="agentdx-case-") as cwd:
            for name, content in (case.get("files") or {}).items():
                p = os.path.join(cwd, name)
                os.makedirs(os.path.dirname(p), exist_ok=True)
                with open(p, "w") as f:
                    f.write(content)
            args = list(case.get("args") or [])
            cmd = self.command(build, wd) + (self.arg_separator if args else []) + args
            code, out, err, to, sec = _exec(cmd, cwd, case.get("stdin", ""), self.run_timeout)
            return RunResult(out, err, code, to, sec)


class PythonRunner(Runner):
    lang, src = "python", "main.py"

    def build(self, wd):
        code, out, err, _, sec = _exec(["python3", "-I", "-m", "py_compile", self.src], wd, timeout=30)
        if code != 0:
            return BuildResult(False, dg.parse_text("python", err), err, sec)
        return BuildResult(True, [], "", sec, prog=os.path.join(wd, self.src))

    def command(self, build, wd):
        return ["python3", "-I", build.prog]


class GoRunner(Runner):
    lang, src = "go", "main.go"

    def build(self, wd):
        out_bin = os.path.join(self.build_dir(wd), "prog")
        env = {"GO111MODULE": "off", "GOFLAGS": ""}
        code, out, err, _, sec = _exec(["go", "build", "-o", out_bin, self.src], wd, timeout=180, env=env)
        if code != 0:
            return BuildResult(False, dg.parse_text("go", out + err), out + err, sec)
        return BuildResult(True, [], "", sec, prog=out_bin)

    def command(self, build, wd):
        return [build.prog]


class RustRunner(Runner):
    lang, src = "rust", "main.rs"

    def build(self, wd):
        out_bin = os.path.join(self.build_dir(wd), "prog")
        code, out, err, _, sec = _exec(["rustc", "--edition", "2021", "-C", "opt-level=1",
                                        "--error-format=json", "-o", out_bin, self.src], wd, timeout=300)
        if code != 0:
            diags = dg.parse_rust_json(err) or dg.parse_text("rust", err)
            return BuildResult(False, diags, err, sec)
        return BuildResult(True, [], "", sec, prog=out_bin)

    def command(self, build, wd):
        return [build.prog]


class TsRunner(Runner):
    lang, src = "ts", "main.ts"

    def build(self, wd):
        import shutil as sh
        t0 = time.time()
        if sh.which("tsc"):
            code, out, err, _, sec = _exec(["tsc", "--noEmit", "--strict", "--target", "es2022",
                                            "--module", "nodenext", "--skipLibCheck", self.src], wd, timeout=120)
            raw = out + err
        elif sh.which("deno"):
            code, out, err, _, sec = _exec(["deno", "check", "--no-lock", self.src], wd, timeout=120,
                                           env={"NO_COLOR": "1"})
            raw = out + err
        else:
            code, out, err, _, sec = _exec(["node", "--check", self.src], wd, timeout=30)
            raw = out + err
        if code != 0:
            return BuildResult(False, dg.parse_text("ts", raw), raw, sec)
        return BuildResult(True, [], "", time.time() - t0, prog=os.path.join(wd, self.src))

    def command(self, build, wd):
        return ["node", "--no-warnings", build.prog]


class WjInterpRunner(Runner):
    lang, src = "wj-interp", "main.wj"
    run_timeout = 20
    arg_separator = ["--"]  # otherwise `wj run` parses program flags such as --name itself

    def build(self, wd):
        # Interpreter has no separate compile step; errors surface at run time (first vector).
        return BuildResult(True, [], "", 0.0, prog=os.path.join(wd, self.src))

    def command(self, build, wd):
        return [wj_bin(), "run", "--interpret", build.prog]


_BUILD_LOCK = threading.Lock()   # cargo builds share one target dir and one binary name: build+copy must not interleave


class WjCompiledRunner(Runner):
    lang, src = "wj", "main.wj"
    build_timeout = 900

    def build(self, wd):
        gen = os.path.join(self.build_dir(wd), "gen")
        shutil.rmtree(gen, ignore_errors=True)
        wj = wj_bin()
        t0 = time.time()
        code, out, err, to, _ = _exec([wj, "build", "--json", "--no-cargo", "-o", gen, self.src], wd, timeout=120)
        diags = []
        try:
            j = json.loads(out[out.index("{"):])
            for d in j.get("diagnostics", []):
                msg = d.get("message", "") if isinstance(d, dict) else str(d)
                c = d.get("code", "") if isinstance(d, dict) else ""
                diags.append({"code": c, "message": msg[:300], "category": dg.categorize("wj", f"{c} {msg}")})
        except ValueError:
            pass
        if code != 0 or not os.path.isfile(os.path.join(gen, "Cargo.toml")):
            if not diags:
                diags = dg.parse_text("wj", out + err)
            return BuildResult(False, diags, out + err, time.time() - t0)
        target = os.environ.get("AGENTDX_CARGO_TARGET") or os.path.expanduser(
            "~/Library/Caches/windjammer/wt-agent-bench")
        env = {"CARGO_TARGET_DIR": target, "CARGO_BUILD_JOBS": os.environ.get("CARGO_BUILD_JOBS", "2")}
        with _BUILD_LOCK:
            return self._cargo_and_copy(wd, gen, target, env, t0)

    def _cargo_and_copy(self, wd, gen, target, env, t0):
        code, out, err, to, _ = _exec(["cargo", "build", "-q", "--message-format=json"], gen,
                                      timeout=self.build_timeout, env=env)
        if code != 0:
            diags = []
            for ln in out.splitlines():
                try:
                    d = json.loads(ln)
                except ValueError:
                    continue
                m = d.get("message") if d.get("reason") == "compiler-message" else None
                if m and m.get("level") == "error":
                    c = (m.get("code") or {}).get("code") or ""
                    diags.append({"code": c, "message": m.get("message", "")[:300],
                                  "detail": (m.get("rendered") or "")[:900],
                                  "category": "toolchain-bug" if c else dg.categorize("wj", m.get("message", ""))})
            if not diags:
                diags = dg.parse_text("wj", err or out)
            return BuildResult(False, diags, (err or out), time.time() - t0)
        import re
        with open(os.path.join(gen, "Cargo.toml")) as f:
            stem = re.search(r'\[\[bin\]\]\s*name = "([^"]+)"', f.read()).group(1)
        prog = os.path.join(self.build_dir(wd), "prog")
        shutil.copy2(os.path.join(target, "debug", stem), prog)
        return BuildResult(True, [], "", time.time() - t0, prog=prog)

    def command(self, build, wd):
        return [build.prog]


RUNNERS = {"python": PythonRunner, "go": GoRunner, "rust": RustRunner, "ts": TsRunner,
           "wj-interp": WjInterpRunner, "wj": WjCompiledRunner}


def get_runner(lang):
    return RUNNERS[lang]()


def classify_run_failure(lang, rr):
    """Run-time failure -> diagnostic dict (or None when the program ran to completion)."""
    if rr.timed_out:
        return {"code": "", "message": "timeout", "category": "timeout"}
    if rr.code != 0:
        text = rr.stderr or rr.stdout
        cat = dg.categorize(lang, text)
        if cat == "other":
            cat = "runtime-panic"
        return {"code": f"exit{rr.code}", "message": text.strip()[-300:], "category": cat}
    return None
