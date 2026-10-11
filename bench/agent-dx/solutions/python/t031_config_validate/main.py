import sys

lines = sys.stdin.read().split("\n")
if lines and lines[-1] == "":
    lines.pop()
errs = []
cfg = {}
unknown = []
FIELDS = ("host", "port", "debug", "workers")
for n, line in enumerate(lines, 1):
    if line == "":
        continue
    if "=" not in line:
        errs.append("error: malformed line %d" % n)
        continue
    k, _, v = line.partition("=")
    cfg[k] = v
    if k not in FIELDS and k not in unknown:
        unknown.append(k)


def parse_int(v, lo, hi):
    if v.isascii() and v.isdigit() and lo <= int(v) <= hi:
        return int(v)
    return None


host = ""
port = None
debug = "false"
workers = 4
if "host" not in cfg:
    errs.append("error: host: missing")
elif cfg["host"] == "":
    errs.append("error: host: empty")
else:
    host = cfg["host"]
if "port" not in cfg:
    errs.append("error: port: missing")
else:
    port = parse_int(cfg["port"], 1, 65535)
    if port is None:
        errs.append("error: port: invalid value " + cfg["port"])
if "debug" in cfg:
    if cfg["debug"] in ("true", "false"):
        debug = cfg["debug"]
    else:
        errs.append("error: debug: invalid value " + cfg["debug"])
if "workers" in cfg:
    workers = parse_int(cfg["workers"], 1, 64)
    if workers is None:
        errs.append("error: workers: invalid value " + cfg["workers"])
for k in unknown:
    errs.append("error: unknown key " + k)
if errs:
    print("\n".join(errs))
else:
    print("ok host=%s port=%d debug=%s workers=%d" % (host, port, debug, workers))
