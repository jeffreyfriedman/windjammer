def _c(stdin): return {"stdin": stdin}
W1 = "room hall\nroom cellar\nroom attic\nexit hall n attic\nexit attic s hall\nexit hall d cellar iron-key\nexit cellar u hall\nthing iron-key 1 attic\nthing lamp 3 hall\nbox chest 10 20 hall\nthing coin 1 in:chest\nthing rock 40 cellar\nplayer hall 12\n---\n"
SAMPLES = [
    _c(W1 + "look\ngo d\ngo n\nlook\ntake iron-key\ngo s\nunlock d\ngo d\nlook\ntake rock\ngo u\nopen chest\ntake coin from chest\ninv\nput lamp in chest\ntake lamp\nput lamp in chest\nlook\n"),
    _c("room a\nroom b\nexit a e b\nexit a e b\nthing x 1 nowhere\nbox bag 5 10 a open\nthing pebble 2 in:bag\nplayer a 6\nplayer b 6\nfoo\n---\nlook\ntake pebble\ntake bag\ninv\nclose bag\nput bag in bag\nfly\n"),
]
EDGE = [
    _c(""),
    _c("room a\n---\nlook\n"),
    _c("room a\nplayer a 5\n"),
    _c("room a\nplayer a 5\n---\nlook\ninv\n\ngo n\ntake\ndrop\nunlock\nopen\n"),
    _c("room a\nbox b1 1 10 a open\nbox b2 1 10 in:b1 open\nbox b3 1 10 in:b2 open\nthing t 2 in:b3\nthing u 3 in:b3\nplayer a 100\n---\nlook\ntake b1\ninv\nput b1 in b3\nput b3 in b1\nput b2 in b2\ndrop b1\nlook\ntake t from b3\nput t in b3\nlook\n"),
    _c("room a\nbox big 5 100 a open\nbox small 5 10 a open\nthing w 8 a\nthing v 3 a\nplayer a 20\n---\nput w in small\ntake w\nput w in small\ntake v\nput v in small\ntake small\ninv\nput big in small\nput small in big\ninv\ndrop big\nlook\ntake small from big\nlook\n"),
    _c("room a\nroom b\nexit a e b k1\nexit b w a\nbox sack 1 50 a open\nthing k1 1 in:sack\nplayer a 100\n---\ngo e\nunlock e\ntake sack\nunlock e\nunlock e\nunlock w\nunlock n\nclose sack\nunlock e\ngo e\ngo e\ngo w\n"),
    _c("room a\nthing hammer 30 a\nthing feather 1 a\nplayer a 30\n---\ntake hammer\ntake feather\ndrop hammer\ntake feather\ntake hammer\ninv\ntake hammer\n"),
    _c("room a\nbox c 2 3 a\nthing x 1 a\nthing y 2 a\nthing z 3 a\nplayer a 50\n---\nput x in c\nopen c\nopen c\nput x in c\nput y in c\nput z in c\nput x in c\nlook\nclose c\nclose c\ntake x from c\nopen c\ntake y from c\ntake y from c\ntake y from x\nlook\ninv\n"),
    _c("room a\nroom b\nroom c\nexit a e b\nexit b e c\nexit c w b\nexit a x c\nexit a n c\nexit a up c\nexit q n a\nthing t 1 a\nthing t 2 b\nthing 1 x b\nbox bx 1 1 in:t\nplayer a 000007\n---\ngo e\ngo e\nlook\ngo w\ngo w\ngo w\n"),
    _c("room cell\nroom yard\nexit cell e yard door-key\nexit cell u yard fake\nthing door-key 1 in:bag\nbox bag 1 5 cell\nthing door-key 1 in:bag\nplayer cell 5\n---\nlook\nunlock e\nopen bag\ntake door-key from bag\nunlock e\nunlock e\nunlock u\nlook\ngo e\ngo w\n"),
]
NAMES = ["lamp", "coin", "key-a", "key-b", "rock", "gem", "book", "sword", "cup"]
BOXES = ["chest", "bag", "crate"]
def rand(rng):
    rooms = rng.sample(["hall", "cave", "attic", "yard", "tower"], rng.randint(3, 5))
    lines = [f"room {r}" for r in rooms]
    dirs = "nsewud"
    opp = {"n": "s", "s": "n", "e": "w", "w": "e", "u": "d", "d": "u"}
    names = rng.sample(NAMES, rng.randint(5, 8))
    keys = [n for n in names if n.startswith("key")]
    used = set()
    for _ in range(rng.randint(len(rooms), len(rooms) * 2)):
        a, b = rng.sample(rooms, 2)
        d = rng.choice(dirs)
        if (a, d) in used or (b, opp[d]) in used: continue
        used.add((a, d)); used.add((b, opp[d]))
        k = f" {rng.choice(keys)}" if keys and rng.random() < 0.35 else ""
        lines.append(f"exit {a} {d} {b}{k}")
        lines.append(f"exit {b} {opp[d]} {a}")
    boxes = rng.sample(BOXES, rng.randint(1, 3))
    placed = []
    for b in boxes:
        loc = (rooms[0] if rng.random() < 0.6 else rng.choice(rooms)) if not placed or rng.random() < 0.6 else f"in:{rng.choice(placed)}"
        lines.append(f"box {b} {rng.randint(1, 5)} {rng.randint(5, 25)} {loc}" + (" open" if rng.random() < 0.5 else ""))
        placed.append(b)
    for n in names:
        if n in placed: continue
        loc = (rooms[0] if rng.random() < 0.6 else rng.choice(rooms)) if rng.random() < 0.65 else f"in:{rng.choice(placed)}"
        lines.append(f"thing {n} {rng.randint(0, 12)} {loc}")
    if rng.random() < 0.15: lines.insert(rng.randint(0, len(lines)), rng.choice(["thing", "room q q", "box b 1", "exit a b"]))
    lines.append(f"player {rooms[0]} {rng.randint(10, 30)}")
    lines.append("---")
    allnames = names + boxes
    world = World(lines)
    cmds = []
    for _ in range(rng.randint(30, 60)):
        vis = world.visible_names()
        vb = [n for n in vis if world.items[n]["box"]]
        pool = vis if vis and rng.random() < 0.85 else allnames
        x = rng.choice(pool)
        bx = rng.choice(vb) if vb and rng.random() < 0.85 else rng.choice(boxes)
        kids = world.children(bx) if bx in world.items else []
        ex = world.exits_here()
        dd = rng.choice(ex) if ex and rng.random() < 0.9 else rng.choice(list(dirs))
        locked = [d for d in ex if (world.player[0], d) in world.locked]
        r = rng.random()
        if r < 0.1: c = "look"
        elif r < 0.15: c = "inv"
        elif r < 0.2: c = f"go {dd}"
        elif r < 0.4: c = f"take {x}"
        elif r < 0.46: c = f"drop {x}"
        elif r < 0.64: c = f"put {x} in {bx}"
        elif r < 0.78: c = f"take {rng.choice(kids) if kids and rng.random() < 0.9 else x} from {bx}"
        elif r < 0.86: c = f"open {bx}"
        elif r < 0.9: c = f"close {bx}"
        elif r < 0.97: c = f"unlock {rng.choice(locked) if locked and rng.random() < 0.9 else dd}"
        else: c = rng.choice(["dance", "go", "take", "put x"])
        cmds.append(c)
        world.run(c)
    return _c("\n".join(lines + cmds) + "\n")

DIRS = "nsewud"
class World:
    def __init__(self, lines):
        self.rooms, self.exits, self.items, self.bad = [], {}, {}, []
        self.player = None
        self.out, self.errs = [], 0
        self.locked = set()
        self.started = False
        for n, ln in enumerate(lines, 1):
            if ln == "": continue
            if not self.world_line(ln): self.err(f"bad world line {n}")
    def err(self, m):
        self.errs += 1; self.out.append("error: " + m)
    def world_line(self, ln):
        p = ln.split(" ")
        if "" in p: return False
        def num(s): return s.isascii() and s.isdigit() and len(s) <= 6
        def loc(s):
            if s.startswith("in:"):
                c = s[3:]
                return ("in", c) if c in self.items and self.items[c]["box"] else None
            return ("room", s) if s in self.rooms else None
        if p[0] == "room" and len(p) == 2 and p[1] not in self.rooms:
            self.rooms.append(p[1]); return True
        if p[0] == "exit" and len(p) in (4, 5):
            if p[1] in self.rooms and p[2] in DIRS and len(p[2]) == 1 and p[3] in self.rooms and (p[1], p[2]) not in self.exits:
                self.exits[(p[1], p[2])] = (p[3], p[4] if len(p) == 5 else None)
                if len(p) == 5: self.locked.add((p[1], p[2]))
                return True
            return False
        if p[0] == "thing" and len(p) == 4 and p[1] not in self.items and num(p[2]) and loc(p[3]):
            self.items[p[1]] = {"w": int(p[2]), "box": False, "open": False, "cap": 0, "loc": loc(p[3])}; return True
        if p[0] == "box" and len(p) in (5, 6) and p[1] not in self.items and num(p[2]) and num(p[3]) and loc(p[4]) and (len(p) == 5 or p[5] == "open"):
            self.items[p[1]] = {"w": int(p[2]), "box": True, "open": len(p) == 6, "cap": int(p[3]), "loc": loc(p[4])}; return True
        if p[0] == "player" and len(p) == 3 and self.player is None and p[1] in self.rooms and num(p[2]):
            self.player = [p[1], int(p[2])]; return True
        return False
    def children(self, b): return sorted(n for n, i in self.items.items() if i["loc"] == ("in", b))
    def total(self, n): return self.items[n]["w"] + sum(self.total(c) for c in self.children(n))
    def carried_direct(self): return sorted(n for n, i in self.items.items() if i["loc"] == ("player", ""))
    def carried(self): return sum(self.total(n) for n in self.carried_direct())
    def room_direct(self): return sorted(n for n, i in self.items.items() if i["loc"] == ("room", self.player[0]))
    def visible_names(self): return self.room_direct() + self.carried_direct()
    def exits_here(self): return [d for d in DIRS if (self.player[0], d) in self.exits]
    def render(self, n):
        i = self.items[n]
        if not i["box"]: return n
        if not i["open"]: return n + "{closed}"
        return n + "{" + ", ".join(self.render(c) for c in self.children(n)) + "}"
    def inside(self, a, b):
        # is a inside (or equal to) b
        cur = a
        while True:
            if cur == b: return True
            l = self.items[cur]["loc"]
            if l[0] != "in": return False
            cur = l[1]
    def run(self, ln):
        if self.player is None: return
        p = ln.split(" ")
        it = self.items
        room, limit = self.player
        c = p[0]
        vis = self.visible_names()
        if c == "look" and len(p) == 1:
            self.out.append(f"== {room} ==")
            ex = [d + ("(locked)" if (room, d) in self.locked else "") for d in self.exits_here()]
            self.out.append("exits: " + (" ".join(ex) if ex else "none"))
            rd = self.room_direct()
            self.out.append("items: " + (", ".join(self.render(n) for n in rd) if rd else "nothing"))
        elif c == "inv" and len(p) == 1:
            cd = self.carried_direct()
            self.out.append("carrying: " + (", ".join(self.render(n) for n in cd) if cd else "nothing") + f" ({self.carried()}/{limit})")
        elif c == "go" and len(p) == 2:
            d = p[1]
            if (room, d) not in self.exits: self.err(f"no exit {d}")
            elif (room, d) in self.locked: self.err(f"door {d} is locked")
            else:
                self.player[0] = self.exits[(room, d)][0]
                self.out.append("-> " + self.player[0])
        elif c == "take" and len(p) == 2:
            x = p[1]
            if x not in self.room_direct(): self.err(f"no {x} here")
            elif self.carried() + self.total(x) > limit: self.err("too heavy")
            else: it[x]["loc"] = ("player", ""); self.out.append(f"taken {x}")
        elif c == "drop" and len(p) == 2:
            x = p[1]
            if x not in self.carried_direct(): self.err(f"not carrying {x}")
            else: it[x]["loc"] = ("room", room); self.out.append(f"dropped {x}")
        elif c == "put" and len(p) == 4 and p[2] == "in":
            x, b = p[1], p[3]
            if x not in vis: self.err(f"no {x} here")
            elif b not in vis: self.err(f"no {b} here")
            elif not it[b]["box"]: self.err(f"{b} is not a box")
            elif self.inside(b, x): self.err(f"cannot put {x} in itself")
            elif not it[b]["open"]: self.err(f"{b} is closed")
            elif sum(self.total(k) for k in self.children(b)) + self.total(x) > it[b]["cap"]: self.err(f"{b} is full")
            else:
                b_carried = it[b]["loc"][0] == "player"
                x_carried = it[x]["loc"][0] == "player"
                if b_carried and not x_carried and self.carried() + self.total(x) > limit: self.err("too heavy")
                else: it[x]["loc"] = ("in", b); self.out.append(f"put {x} in {b}")
        elif c == "take" and len(p) == 4 and p[2] == "from":
            x, b = p[1], p[3]
            if b not in vis: self.err(f"no {b} here")
            elif not it[b]["box"]: self.err(f"{b} is not a box")
            elif not it[b]["open"]: self.err(f"{b} is closed")
            elif x not in self.children(b): self.err(f"{x} is not in {b}")
            elif it[b]["loc"][0] != "player" and self.carried() + self.total(x) > limit: self.err("too heavy")
            else: it[x]["loc"] = ("player", ""); self.out.append(f"taken {x} from {b}")
        elif c in ("open", "close") and len(p) == 2:
            b = p[1]
            if b not in vis: self.err(f"no {b} here")
            elif not it[b]["box"]: self.err(f"{b} is not a box")
            elif it[b]["open"] == (c == "open"): self.err(f"{b} is already {'open' if c == 'open' else 'closed'}")
            else: it[b]["open"] = c == "open"; self.out.append(f"{'opened' if c == 'open' else 'closed'} {b}")
        elif c == "unlock" and len(p) == 2:
            d = p[1]
            if (room, d) not in self.exits: self.err(f"no exit {d}")
            elif (room, d) not in self.locked: self.err(f"{d} is not locked")
            else:
                key = self.exits[(room, d)][1]
                if key not in it or not any(self.inside(key, n) for n in self.carried_direct()): self.err(f"need {key}")
                else: self.locked.discard((room, d)); self.out.append(f"unlocked {d}")
        else:
            self.err("bad command")
def solve(case):
    lines = case["stdin"].split("\n")
    if lines and lines[-1] == "": lines.pop()
    if "---" in lines:
        i = lines.index("---"); wl, cl = lines[:i], lines[i + 1:]
    else: wl, cl = lines, []
    w = World(wl)
    if w.player is None:
        w.err("no player")
    else:
        for ln in cl:
            if ln != "": w.run(ln)
    return "".join(x + "\n" for x in w.out), (1 if w.errs else 0)
