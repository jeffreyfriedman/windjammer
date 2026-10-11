def _c(lines):
    return {"stdin": "".join(l + "\n" for l in lines)}


SAMPLES = [_c(['a,b,c', '"x,y",z']), _c(['', ',', '"he said ""hi""",1'])]
EDGE = [_c(['""', '"",""', 'a,,b', '"a""",b', ',,,', ' lead, trail ']), _c(['"""",x', '"a,b,c,d"'])]


def _field(rng):
    k = rng.random()
    s = "".join(rng.choice("ab ,\"x1") for _ in range(rng.randint(0, 5)))
    if k < 0.5 and "," not in s and '"' not in s:
        return s
    return '"' + s.replace('"', '""') + '"'


def rand(rng):
    return _c([",".join(_field(rng) for _ in range(rng.randint(1, 5))) for _ in range(rng.randint(1, 6))])


def parse(line):
    fields, cur, i, n = [], "", 0, len(line)
    while True:
        if i < n and line[i] == '"':
            i += 1
            while True:
                if line[i] == '"':
                    if i + 1 < n and line[i + 1] == '"':
                        cur += '"'
                        i += 2
                        continue
                    i += 1
                    break
                cur += line[i]
                i += 1
        else:
            while i < n and line[i] != ",":
                cur += line[i]
                i += 1
        fields.append(cur)
        cur = ""
        if i >= n:
            return fields
        i += 1  # comma
        if i >= n:
            fields.append("")
            return fields


def solve(case):
    out = []
    for ln in case["stdin"].split("\n")[:-1]:
        f = parse(ln)
        out.append(f"{len(f)}: " + "|".join(f"[{x}]" for x in f))
    return "\n".join(out) + "\n"
