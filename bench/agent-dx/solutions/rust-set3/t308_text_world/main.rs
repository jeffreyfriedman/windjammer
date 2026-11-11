use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Read};

#[derive(Clone, PartialEq, Eq, Debug)]
enum Loc {
    Room(String),
    In(String),
    Player,
}

struct Item {
    weight: u64,
    cap: u64,
    is_box: bool,
    open: bool,
    loc: Loc,
}

struct Exit {
    to: String,
    key: Option<String>,
    locked: bool,
}

struct Player {
    room: String,
    limit: u64,
}

struct World {
    rooms: HashSet<String>,
    exits: HashMap<(String, char), Exit>,
    items: BTreeMap<String, Item>,
    player: Option<Player>,
    out: Vec<String>,
    errors: usize,
}

const DIRS: [char; 6] = ['n', 's', 'e', 'w', 'u', 'd'];

fn number(s: &str) -> Option<u64> {
    if s.is_empty() || s.len() > 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        s.parse().ok()
    }
}

impl World {
    fn err(&mut self, m: String) {
        self.errors += 1;
        self.out.push(format!("error: {}", m));
    }

    fn location(&self, s: &str) -> Option<Loc> {
        if let Some(b) = s.strip_prefix("in:") {
            match self.items.get(b) {
                Some(i) if i.is_box => Some(Loc::In(b.to_string())),
                _ => None,
            }
        } else if self.rooms.contains(s) {
            Some(Loc::Room(s.to_string()))
        } else {
            None
        }
    }

    fn world_line(&mut self, line: &str) -> bool {
        let p: Vec<&str> = line.split(' ').collect();
        if p.iter().any(|t| t.is_empty()) {
            return false;
        }
        match (p[0], p.len()) {
            ("room", 2) => self.rooms.insert(p[1].to_string()),
            ("exit", 4) | ("exit", 5) => {
                let dir = match p[2].chars().collect::<Vec<_>>().as_slice() {
                    [c] if DIRS.contains(c) => *c,
                    _ => return false,
                };
                if !self.rooms.contains(p[1]) || !self.rooms.contains(p[3]) || self.exits.contains_key(&(p[1].to_string(), dir)) {
                    return false;
                }
                let key = p.get(4).map(|k| k.to_string());
                let locked = key.is_some();
                self.exits.insert((p[1].to_string(), dir), Exit { to: p[3].to_string(), key, locked });
                true
            }
            ("thing", 4) | ("box", 5) | ("box", 6) => {
                let is_box = p[0] == "box";
                if self.items.contains_key(p[1]) {
                    return false;
                }
                let weight = match number(p[2]) {
                    Some(w) => w,
                    None => return false,
                };
                let (cap, loc_s) = if is_box {
                    match number(p[3]) {
                        Some(c) => (c, p[4]),
                        None => return false,
                    }
                } else {
                    (0, p[3])
                };
                let open = if is_box && p.len() == 6 {
                    if p[5] != "open" {
                        return false;
                    }
                    true
                } else {
                    false
                };
                match self.location(loc_s) {
                    Some(loc) => {
                        self.items.insert(p[1].to_string(), Item { weight, cap, is_box, open, loc });
                        true
                    }
                    None => false,
                }
            }
            ("player", 3) => {
                if self.player.is_some() || !self.rooms.contains(p[1]) {
                    return false;
                }
                match number(p[2]) {
                    Some(limit) => {
                        self.player = Some(Player { room: p[1].to_string(), limit });
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn children(&self, b: &str) -> Vec<&String> {
        self.items.iter().filter(|(_, i)| i.loc == Loc::In(b.to_string())).map(|(n, _)| n).collect()
    }

    fn total(&self, n: &str) -> u64 {
        self.items[n].weight + self.children(n).iter().map(|c| self.total(c)).sum::<u64>()
    }

    fn direct(&self, loc: &Loc) -> Vec<&String> {
        self.items.iter().filter(|(_, i)| &i.loc == loc).map(|(n, _)| n).collect()
    }

    fn room_loc(&self) -> Loc {
        Loc::Room(self.player.as_ref().unwrap().room.clone())
    }

    fn carried(&self) -> u64 {
        self.direct(&Loc::Player).iter().map(|n| self.total(n)).sum()
    }

    fn visible(&self, n: &str) -> bool {
        match self.items.get(n) {
            Some(i) => i.loc == Loc::Player || i.loc == self.room_loc(),
            None => false,
        }
    }

    fn render(&self, n: &str) -> String {
        let i = &self.items[n];
        if !i.is_box {
            n.to_string()
        } else if !i.open {
            format!("{}{{closed}}", n)
        } else {
            let inner: Vec<String> = self.children(n).iter().map(|c| self.render(c)).collect();
            format!("{}{{{}}}", n, inner.join(", "))
        }
    }

    fn render_list(&self, names: &[&String]) -> String {
        names.iter().map(|n| self.render(n)).collect::<Vec<_>>().join(", ")
    }

    // is `a` equal to `b` or (transitively) inside it?
    fn inside(&self, a: &str, b: &str) -> bool {
        let mut cur = a;
        loop {
            if cur == b {
                return true;
            }
            match &self.items[cur].loc {
                Loc::In(p) => cur = p,
                _ => return false,
            }
        }
    }

    fn command(&mut self, line: &str) {
        let p: Vec<&str> = line.split(' ').collect();
        let (room, limit) = {
            let pl = self.player.as_ref().unwrap();
            (pl.room.clone(), pl.limit)
        };
        match (p[0], p.len()) {
            ("look", 1) => {
                let ex: Vec<String> = DIRS
                    .iter()
                    .filter_map(|d| self.exits.get(&(room.clone(), *d)).map(|e| format!("{}{}", d, if e.locked { "(locked)" } else { "" })))
                    .collect();
                let here = self.direct(&self.room_loc());
                let items = if here.is_empty() { "nothing".to_string() } else { self.render_list(&here) };
                let ex_s = if ex.is_empty() { "none".to_string() } else { ex.join(" ") };
                self.out.push(format!("== {} ==", room));
                self.out.push(format!("exits: {}", ex_s));
                self.out.push(format!("items: {}", items));
            }
            ("inv", 1) => {
                let held = self.direct(&Loc::Player);
                let s = if held.is_empty() { "nothing".to_string() } else { self.render_list(&held) };
                let line = format!("carrying: {} ({}/{})", s, self.carried(), limit);
                self.out.push(line);
            }
            ("go", 2) => {
                let d = p[1].chars().next().filter(|_| p[1].len() == 1);
                let ex = d.and_then(|d| self.exits.get(&(room.clone(), d)));
                match ex {
                    None => self.err(format!("no exit {}", p[1])),
                    Some(e) if e.locked => self.err(format!("door {} is locked", p[1])),
                    Some(e) => {
                        let to = e.to.clone();
                        self.player.as_mut().unwrap().room = to.clone();
                        self.out.push(format!("-> {}", to));
                    }
                }
            }
            ("take", 2) => {
                let x = p[1];
                if !self.items.get(x).map_or(false, |i| i.loc == Loc::Room(room.clone())) {
                    self.err(format!("no {} here", x));
                } else if self.carried() + self.total(x) > limit {
                    self.err("too heavy".to_string());
                } else {
                    self.items.get_mut(x).unwrap().loc = Loc::Player;
                    self.out.push(format!("taken {}", x));
                }
            }
            ("drop", 2) => {
                let x = p[1];
                if !self.items.get(x).map_or(false, |i| i.loc == Loc::Player) {
                    self.err(format!("not carrying {}", x));
                } else {
                    self.items.get_mut(x).unwrap().loc = Loc::Room(room.clone());
                    self.out.push(format!("dropped {}", x));
                }
            }
            ("put", 4) if p[2] == "in" => self.put(p[1], p[3], limit),
            ("take", 4) if p[2] == "from" => {
                let (x, b) = (p[1], p[3]);
                if !self.visible(b) {
                    self.err(format!("no {} here", b));
                } else if !self.items[b].is_box {
                    self.err(format!("{} is not a box", b));
                } else if !self.items[b].open {
                    self.err(format!("{} is closed", b));
                } else if !self.items.get(x).map_or(false, |i| i.loc == Loc::In(b.to_string())) {
                    self.err(format!("{} is not in {}", x, b));
                } else if self.items[b].loc != Loc::Player && self.carried() + self.total(x) > limit {
                    self.err("too heavy".to_string());
                } else {
                    self.items.get_mut(x).unwrap().loc = Loc::Player;
                    self.out.push(format!("taken {} from {}", x, b));
                }
            }
            ("open", 2) | ("close", 2) => {
                let b = p[1];
                let want = p[0] == "open";
                if !self.visible(b) {
                    self.err(format!("no {} here", b));
                } else if !self.items[b].is_box {
                    self.err(format!("{} is not a box", b));
                } else if self.items[b].open == want {
                    self.err(format!("{} is already {}", b, p[0].replace("close", "closed")));
                } else {
                    self.items.get_mut(b).unwrap().open = want;
                    self.out.push(format!("{} {}", if want { "opened" } else { "closed" }, b));
                }
            }
            ("unlock", 2) => {
                let d = p[1].chars().next().filter(|_| p[1].len() == 1);
                let key = match d.and_then(|d| self.exits.get(&(room.clone(), d))) {
                    None => return self.err(format!("no exit {}", p[1])),
                    Some(e) if !e.locked => return self.err(format!("{} is not locked", p[1])),
                    Some(e) => e.key.clone().unwrap(),
                };
                let carrying = self.items.contains_key(&key) && self.direct(&Loc::Player).iter().any(|n| self.inside(&key, n));
                if !carrying {
                    self.err(format!("need {}", key));
                } else {
                    self.exits.get_mut(&(room.clone(), d.unwrap())).unwrap().locked = false;
                    self.out.push(format!("unlocked {}", p[1]));
                }
            }
            _ => self.err("bad command".to_string()),
        }
    }

    fn put(&mut self, x: &str, b: &str, limit: u64) {
        if !self.visible(x) {
            return self.err(format!("no {} here", x));
        }
        if !self.visible(b) {
            return self.err(format!("no {} here", b));
        }
        if !self.items[b].is_box {
            return self.err(format!("{} is not a box", b));
        }
        if self.inside(b, x) {
            return self.err(format!("cannot put {} in itself", x));
        }
        if !self.items[b].open {
            return self.err(format!("{} is closed", b));
        }
        let content: u64 = self.children(b).iter().map(|c| self.total(c)).sum();
        if content + self.total(x) > self.items[b].cap {
            return self.err(format!("{} is full", b));
        }
        if self.items[b].loc == Loc::Player && self.items[x].loc != Loc::Player && self.carried() + self.total(x) > limit {
            return self.err("too heavy".to_string());
        }
        self.items.get_mut(x).unwrap().loc = Loc::In(b.to_string());
        self.out.push(format!("put {} in {}", x, b));
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let lines: Vec<&str> = input.lines().collect();
    let split = lines.iter().position(|l| *l == "---");
    let (world_lines, cmd_lines) = match split {
        Some(i) => (&lines[..i], &lines[i + 1..]),
        None => (&lines[..], &lines[0..0]),
    };
    let mut w = World { rooms: HashSet::new(), exits: HashMap::new(), items: BTreeMap::new(), player: None, out: Vec::new(), errors: 0 };
    for (n, l) in world_lines.iter().enumerate() {
        if !l.is_empty() && !w.world_line(l) {
            w.err(format!("bad world line {}", n + 1));
        }
    }
    if w.player.is_none() {
        w.err("no player".to_string());
    } else {
        for l in cmd_lines {
            if !l.is_empty() {
                w.command(l);
            }
        }
    }
    for l in &w.out {
        println!("{}", l);
    }
    std::process::exit(if w.errors > 0 { 1 } else { 0 });
}
