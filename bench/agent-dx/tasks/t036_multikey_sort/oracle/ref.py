def _c(spec, recs):
    return {"stdin": spec + "\n" + "".join(f"{n} {a} {c}\n" for n, a, c in recs)}


R = [("bob", 30, "paris"), ("amy", 25, "rome"), ("cat", 30, "oslo"), ("dan", 25, "rome"), ("amy", 31, "bern")]
SAMPLES = [_c("age:desc,name:asc", R), _c("city:asc", R)]
EDGE = [_c("name:asc", []), _c("age:asc", [("a", 1, "x"), ("b", 1, "x"), ("c", 1, "x")]), _c("age:desc,city:desc,name:desc", R), _c("name:desc,age:asc", R + [("amy", 25, "zzz")]), _c("age:asc", [("n", 9, "c"), ("m", 10, "c"), ("l", -1, "c")])]


def rand(rng):
    recs = [(rng.choice(["ann", "bob", "cy", "Di"]), rng.randint(-3, 40), rng.choice(["x", "yy", "Zed", "a"])) for _ in range(rng.randint(0, 12))]
    keys = rng.sample(["name", "age", "city"], rng.randint(1, 3))
    return _c(",".join(f"{k}:{rng.choice(['asc', 'desc'])}" for k in keys), recs)


def solve(case):
    lines = case["stdin"].split("\n")[:-1]
    keys = [k.split(":") for k in lines[0].split(",")]
    recs = [l.split() for l in lines[1:]]
    idx = {"name": 0, "age": 1, "city": 2}
    for f, d in reversed(keys):
        i = idx[f]
        recs.sort(key=(lambda r: int(r[1])) if i == 1 else (lambda r, i=i: r[i].encode()), reverse=(d == "desc"))
    return "".join(" ".join(r) + "\n" for r in recs)
