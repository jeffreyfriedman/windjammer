import zlib
def _c(stdin): return {"stdin": stdin}
SAMPLES = [_c("hello\n123456789\n"), _c("The quick brown fox\n")]
EDGE = [_c(""), _c("\n"), _c("a\n\nb\n"), _c("no newline at end"), _c("café üñ\n"), _c("x" * 300 + "\n")]
def rand(rng):
    alpha = "abcdefXYZ 0123456789-_.,;:!?éß"
    return _c("".join("".join(rng.choice(alpha) for _ in range(rng.randint(0, 25))) + "\n" for _ in range(rng.randint(1, 6))))
def solve(case):
    lines = case["stdin"].split("\n")
    if lines and lines[-1] == "": lines.pop()
    return "".join(f"{zlib.crc32(l.encode()) & 0xffffffff:08x}\n" for l in lines) + f"lines: {len(lines)}\n"
