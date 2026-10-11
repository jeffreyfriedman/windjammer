"""Output comparison shared by all languages (the grading rule)."""


def normalize(s):
    lines = [ln.rstrip() for ln in s.replace("\r\n", "\n").split("\n")]
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines)


def same(expected, actual):
    return normalize(expected) == normalize(actual)
