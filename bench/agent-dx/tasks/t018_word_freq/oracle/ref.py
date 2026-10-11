import re
from collections import Counter

WORDS = "the a cat dog Dog THE run Run it's 42 x9 and or if".split()


def _c(k, text):
    return {"stdin": f"{k}\n{text}"}


SAMPLES = [_c(2, "the cat and the dog. The end!\n"), _c(5, "a b a\nB c\n")]
EDGE = [_c(3, ""), _c(1, "...!!!\n"), _c(10, "Don't stop: don't\n"), _c(2, "x9 X9 x_9 9x 42 042\n")]


def rand(rng):
    text = ""
    for _ in range(rng.randint(1, 6)):
        text += (" " + rng.choice([",", ".", " ", "-", "\n", ""])).join(rng.choice(WORDS) for _ in range(rng.randint(0, 8))) + "\n"
    return _c(rng.randint(1, 6), text)


def solve(case):
    k, _, text = case["stdin"].partition("\n")
    c = Counter(w.lower() for w in re.findall(r"[A-Za-z0-9]+", text))
    items = sorted(c.items(), key=lambda kv: (-kv[1], kv[0]))[: int(k)]
    return "".join(f"{w} {n}\n" for w, n in items)
