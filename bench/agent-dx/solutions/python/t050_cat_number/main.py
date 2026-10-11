import sys

args = sys.argv[1:]
number = "-n" in args
files = [a for a in args if a != "-n"]
out = []
status = 0
num = 0


def emit(data):
    global num
    if not number:
        out.append(data)
        return
    for line in data.splitlines(keepends=True):
        num += 1
        if not line.endswith(b"\n"):
            line += b"\n"
        out.append(b"%6d\t" % num + line)


if not files:
    emit(sys.stdin.buffer.read())
for name in files:
    try:
        with open(name, "rb") as f:
            data = f.read()
    except OSError:
        out.append(("cat: %s: No such file\n" % name).encode())
        status = 1
        continue
    emit(data)
sys.stdout.buffer.write(b"".join(out))
sys.exit(status)
