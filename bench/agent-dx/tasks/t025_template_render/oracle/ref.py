import re


def _c(defs, tpl):
    return {"stdin": f"{len(defs)}\n" + "".join(f"{k}={v}\n" for k, v in defs) + tpl}


SAMPLES = [_c([("name", "World")], "Hello, {{name}}!\n"), _c([("a", "1"), ("b", "x=y")], "{{a}}+{{ b }}={{c}}\n")]
EDGE = [_c([], ""), _c([("x", "")], "[{{x}}]{{ x}}{{x }}{{  x  }}\n"), _c([("x", "{{x}}")], "{{x}} { {{ x y }} {{}} {{x\n"),
        _c([("a_1", "ok")], "{{a_1}}{{a_1}}\nline2 {{A_1}}\n")]


def rand(rng):
    keys = ["a", "bb", "c_3", "D"]
    defs = [(k, "".join(rng.choice("xyz={} 1") for _ in range(rng.randint(0, 4)))) for k in rng.sample(keys, rng.randint(0, 3))]
    parts = []
    for _ in range(rng.randint(1, 8)):
        parts.append(rng.choice(["{{a}}", "{{ bb }}", "{{c_3}}", "{{D}}", "{{ z }}", "{", "}}", "text ", "\n", "{{ a b }}"]))
    return _c(defs, "".join(parts) + "\n")


def solve(case):
    lines = case["stdin"].split("\n")
    k = int(lines[0])
    d = {}
    for l in lines[1:1 + k]:
        a, _, b = l.partition("=")
        d[a] = b
    tpl = "\n".join(lines[1 + k:])
    return re.sub(r"\{\{ *(\w+) *\}\}", lambda m: d.get(m.group(1), f"<missing:{m.group(1)}>"), tpl)
