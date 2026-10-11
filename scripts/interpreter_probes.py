import subprocess, sys, tempfile, os
wj = sys.argv[1]
probes = [
 ("struct + impl method", 'struct P { x: int }\nimpl P { fn get(self) -> int { self.x } }\nfn main() { let p = P { x: 3 }\n println("{}", p.get()) }', "3\n"),
 ("method mutates self", 'struct C { n: int }\nimpl C { fn inc(self) { self.n += 1 } }\nfn main() { let mut c = C { n: 0 }\n c.inc()\n c.inc()\n println("{}", c.n) }', "2\n"),
 ("enum + match data", 'enum S { A(int), B }\nfn f(s: S) -> int { match s { S::A(n) => n * 2, S::B => 0 } }\nfn main() { println("{}", f(S::A(4))) }', "8\n"),
 ("trait default method", 'trait T { fn a(self) -> int\n fn b(self) -> int { self.a() + 1 } }\nstruct X {}\nimpl T for X { fn a(self) -> int { 1 } }\nfn main() { let x = X {}\n println("{}", x.b()) }', "2\n"),
 ("generic fn", 'fn id<T>(x: T) -> T { x }\nfn main() { println("{}", id(5)) }', "5\n"),
 ("closure call", 'fn main() { let k = 3\n let f = |x| x + k\n println("{}", f(4)) }', "7\n"),
 ("closure mutating capture", 'fn main() { let mut c = 0\n let mut f = || { c += 1 }\n f()\n f()\n println("{}", c) }', "2\n"),
 ("string interpolation", 'fn main() { let n = 5\n println("v=${n + 1}") }', "v=6\n"),
 ("format width/precision", 'fn main() { println("{:>4}|{:.1}", 7, 2.55) }', "   7|2.5\n"),
 ("{:?} on Vec", 'fn main() { let v = vec![1, 2]\n println("{:?}", v) }', "[1, 2]\n"),
 ("Vec push/len/index", 'fn main() { let mut v = vec![1]\n v.push(5)\n println("{} {}", v.len(), v[1]) }', "2 5\n"),
 ("Vec iter map sum", 'fn main() { let v = vec![1, 2, 3]\n let s: int = v.iter().map(|x| x * 2).sum()\n println("{}", s) }', "12\n"),
 ("HashMap insert/get", 'use std::collections::HashMap\nfn main() { let mut m = HashMap::new()\n m.insert("a", 1)\n println("{:?}", m.get("a")) }', "Some(1)\n"),
 ("Option unwrap_or/map", 'fn main() { let o = Some(2)\n println("{}", o.map(|x| x + 1).unwrap_or(0)) }', "3\n"),
 ("Result + ?", 'fn f(x: int) -> Result<int, string> { if x > 0 { Ok(x) } else { Err("neg".to_string()) } }\nfn g() -> Result<int, string> { let a = f(1)?\n let b = f(-1)?\n Ok(a + b) }\nfn main() { println("{:?}", g()) }', 'Err("neg")\n'),
 ("while let / pop", 'fn main() { let mut s = vec![1, 2]\n while let Some(x) = s.pop() { print("{}", x) }\n println("") }', "21\n"),
 ("for over range + continue", 'fn main() { let mut t = 0\n for i in 0..10 { if i % 2 == 0 { continue }\n t += i }\n println("{}", t) }', "25\n"),
 ("i32 overflow panics", 'fn main() { let x: i32 = 2147483647\n let y = x + 1\n println("{}", y) }', "PANIC"),
 ("u8 wrapping_add", 'fn main() { let b: u8 = 200\n println("{}", b.wrapping_add(100)) }', "44\n"),
 ("as casts wrap", 'fn main() { let n: i32 = 300\n println("{}", n as u8) }', "44\n"),
 ("string methods", 'fn main() { println("{}", "a,b".split(",").count()) }', "2\n"),
 ("pipe operator", 'fn dbl(x: int) -> int { x * 2 }\nfn main() { println("{}", 3 |> dbl) }', "6\n"),
 ("process exit code on panic", 'fn main() { panic("boom") }', "EXIT101"),
 ("index OOB panics", 'fn main() { let v = vec![1]\n println("{}", v[3]) }', "PANIC"),
 ("unknown fn is an error", 'fn main() { println("{}", nope(1)) }', "ERROR"),
 ("tuple destructure", 'fn main() { let (a, b) = (1, 2)\n println("{}", a + b) }', "3\n"),
 ("shadowing + blocks", 'fn main() { let x = 1\n let x = x + 1\n { let x = 10 }\n println("{}", x) }', "2\n"),
 ("struct Display via ${}", 'struct P { x: int }\nfn main() { let p = P { x: 1 }\n println("{}", p.x) }', "1\n"),
 ("sort + dedup", 'fn main() { let mut v = vec![3, 1, 3]\n v.sort()\n v.dedup()\n println("{:?}", v) }', "[1, 3]\n"),
 ("iter enumerate/zip", 'fn main() { let v = vec![5, 6]\n for (i, x) in v.iter().enumerate() { print("{}{} ", i, x) }\n println("") }', "05 16 \n"),
]
ok = 0
for name, src, want in probes:
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "p.wj"); open(p, "w").write(src)
        try:
            r = subprocess.run([wj, "run", "--interpret", p], capture_output=True, text=True, timeout=20)
        except subprocess.TimeoutExpired:
            print("TIMEOUT", name); continue
        out = r.stdout
        lines = out.splitlines(keepends=True)
        if lines and lines[0].startswith("Interpreting "): out = "".join(lines[1:])
        if want == "PANIC": good = r.returncode == 101 or (r.returncode != 0 and "panic" in r.stderr.lower())
        elif want == "EXIT101": good = r.returncode == 101
        elif want == "ERROR": good = r.returncode != 0
        else: good = out == want and r.returncode == 0
        ok += good
        print(("PASS " if good else "FAIL ") + name + ("" if good else "   got=%r exit=%d" % (out[:60], r.returncode)))
print("%d/%d" % (ok, len(probes)))
