use std::io::Read;

struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
    sets: usize,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu { parent: (0..n).collect(), size: vec![1; n], sets: n }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        let mut c = x;
        while self.parent[c] != r {
            let n = self.parent[c];
            self.parent[c] = r;
            c = n;
        }
        r
    }

    fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        let (big, small) = if self.size[ra] >= self.size[rb] { (ra, rb) } else { (rb, ra) };
        self.parent[small] = big;
        self.size[big] += self.size[small];
        self.sets -= 1;
        true
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let n: usize = lines.next().unwrap().trim().parse().unwrap();
    let mut d = Dsu::new(n);
    for line in lines {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        let a: usize = p[1].parse().unwrap();
        match p[0] {
            "union" => {
                let b: usize = p[2].parse().unwrap();
                println!("{}", if d.union(a, b) { "merged" } else { "same" });
            }
            "find" => {
                let b: usize = p[2].parse().unwrap();
                println!("{}", if d.find(a) == d.find(b) { "yes" } else { "no" });
            }
            "size" => {
                let r = d.find(a);
                println!("{}", d.size[r]);
            }
            _ => {}
        }
    }
    println!("components {}", d.sets);
}
