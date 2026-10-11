def _c(w, h, g, rows):
    return {"stdin": f"{w} {h} {g}\n" + "".join(r + "\n" for r in rows)}


SAMPLES = [_c(5, 5, 1, [".....", "..#..", "..#..", "..#..", "....."]), _c(4, 4, 4, [".#..", "..#.", "###.", "...."])]
EDGE = [_c(1, 1, 3, ["#"]), _c(2, 2, 2, ["##", "##"]), _c(3, 1, 5, ["###"]), _c(6, 6, 0, ["#....#", "......", "......", "......", "......", "#....#"]), _c(30, 30, 200, ["#" * 30] + ["." * 30] * 29)]


def rand(rng):
    w, h = rng.randint(1, 12), rng.randint(1, 12)
    return _c(w, h, rng.randint(0, 25), ["".join(rng.choice("#.") for _ in range(w)) for _ in range(h)])


def solve(case):
    ls = case["stdin"].split("\n")
    w, h, g = map(int, ls[0].split())
    grid = [[c == "#" for c in ls[1 + y]] for y in range(h)]
    for _ in range(g):
        new = [[False] * w for _ in range(h)]
        for y in range(h):
            for x in range(w):
                n = sum(grid[(y + dy) % h][(x + dx) % w] for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dy, dx) != (0, 0))
                new[y][x] = n == 3 or (grid[y][x] and n == 2)
        grid = new
    out = ["".join("#" if c else "." for c in r) for r in grid]
    out.append(f"alive: {sum(map(sum, grid))}")
    return "\n".join(out) + "\n"
