import json
def _c(stdin): return {"stdin": stdin}
def _o(i, cust, items, note=None):
    d = {"id": i, "customer": cust, "items": [{"sku": s, "qty": q, "price": p} for s, q, p in items]}
    if note is not None: d["note"] = note
    return d
SAMPLES = [
  _c(json.dumps([_o(1, "ann", [("A", 2, 3.5), ("B", 1, 10)]), _o(2, "bob", [("A", 1, 3.5)], "rush"), _o(3, "ann", [("C", 4, 0.25)])])),
  _c(json.dumps([_o(7, "zed", [])])),
]
EDGE = [_c("[]"), _c(""), _c("{"), _c('[{"id":1,"customer":"x","items":[{"sku":"a","qty":"two","price":1}]}]'),
        _c(json.dumps([_o(1, "Ann", [("A", 1, 1.005)]), _o(2, "ann", [("A", 3, 0.1)])])),
        _c(json.dumps([_o(1, "b", [("A", 1000000, 1999.99)])]))]
def rand(rng):
    names = ["ann", "bob", "cy", "Dee", "eve", "zed"]
    orders = []
    for i in range(rng.randint(1, 6)):
        items = [(rng.choice("ABCD"), rng.randint(0, 9), rng.choice([0.5, 1.25, 3, 7.99, 12.5, 0.1])) for _ in range(rng.randint(0, 4))]
        orders.append(_o(i, rng.choice(names), items, rng.choice([None, None, "n"])))
    return _c(json.dumps(orders, indent=rng.choice([None, 2])))
def solve(case):
    try:
        data = json.loads(case["stdin"])
        tot = {}
        assert isinstance(data, list)
        for o in data:
            assert isinstance(o["id"], int) and isinstance(o["customer"], str)
            assert set(o) <= {"id", "customer", "items", "note"}
            t = tot.setdefault(o["customer"], 0.0)
            for it in o["items"]:
                assert isinstance(it["sku"], str) and isinstance(it["qty"], int) and not isinstance(it["qty"], bool)
                assert isinstance(it["price"], (int, float)) and not isinstance(it["price"], bool)
                t += it["qty"] * it["price"]
            tot[o["customer"]] = t
    except Exception:
        return "invalid input\n", 1
    return "".join(f"{k} {tot[k]:.2f}\n" for k in sorted(tot, key=lambda s: s.encode()))
