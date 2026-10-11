def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(["host=example.org", "port=8080"]), _c(["port=0", "debug=maybe", "color=red", "workers=64"])]
EDGE = [_c([]), _c(["host=", "port=65535", "workers=65", "workers=3", "workers=x"]), _c(["nonsense", "", "host=a=b", "port=1", "debug=true", "debug=false"]),
        _c(["host=h", "port=+5", "port=05"]), _c(["host=h", "port=99999", "zeta=1", "alpha=2", "zeta=3", "oops"])]


def rand(rng):
    lines = []
    for _ in range(rng.randint(0, 8)):
        k = rng.choice(["host", "port", "debug", "workers", "host", "port", "extra"])
        v = {"host": rng.choice(["", "h", "a.b"]), "port": rng.choice(["80", "0", "70000", "x", "443", ""]),
             "debug": rng.choice(["true", "false", "yes", ""]), "workers": rng.choice(["1", "64", "0", "65", "4", "z"]),
             "extra": "1"}[k]
        lines.append(f"{k}={v}" if rng.random() < 0.9 else k)
    return _c(lines)


def solve(case):
    lines = case["stdin"].split("\n")[:-1]
    mal, d, order = [], {}, []
    for i, ln in enumerate(lines, 1):
        if ln == "":
            continue
        if "=" not in ln:
            mal.append(f"error: malformed line {i}")
            continue
        k, _, v = ln.partition("=")
        if k not in d:
            order.append(k)
        d[k] = v
    errs = list(mal)

    def isint(v, lo, hi):
        return v.isascii() and v.isdigit() and lo <= int(v) <= hi
    if "host" not in d:
        errs.append("error: host: missing")
    elif d["host"] == "":
        errs.append("error: host: empty")
    if "port" not in d:
        errs.append("error: port: missing")
    elif not isint(d["port"], 1, 65535):
        errs.append(f"error: port: invalid value {d['port']}")
    if "debug" in d and d["debug"] not in ("true", "false"):
        errs.append(f"error: debug: invalid value {d['debug']}")
    if "workers" in d and not isint(d["workers"], 1, 64):
        errs.append(f"error: workers: invalid value {d['workers']}")
    for k in order:
        if k not in ("host", "port", "debug", "workers"):
            errs.append(f"error: unknown key {k}")
    if errs:
        return "\n".join(errs) + "\n"
    return f"ok host={d['host']} port={int(d['port'])} debug={d.get('debug', 'false')} workers={int(d.get('workers', '4'))}\n"
