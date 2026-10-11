import * as fs from "node:fs";

class Memo<A, R> {
  private cache = new Map<A, R>();
  hits = 0;
  misses = 0;
  private fn: (a: A) => R;
  constructor(fn: (a: A) => R) {
    this.fn = fn;
  }
  get(a: A): { value: R; hit: boolean } {
    if (this.cache.has(a)) {
      this.hits++;
      return { value: this.cache.get(a)!, hit: true };
    }
    this.misses++;
    const value = this.fn(a);
    this.cache.set(a, value);
    return { value, hit: false };
  }
}

const MOD = 1000000007n;
const fns: Record<string, Memo<bigint, bigint>> = {
  fib: new Memo((n) => {
    let a = 0n, b = 1n;
    for (let i = 0n; i < n; i++) [a, b] = [b, (a + b) % MOD];
    return a;
  }),
  collatz: new Memo((n) => {
    let s = 0n;
    while (n !== 1n) {
      n = n % 2n === 0n ? n / 2n : 3n * n + 1n;
      s++;
    }
    return s;
  }),
  tri: new Memo((n) => (n * (n + 1n)) / 2n),
};

let hits = 0, misses = 0;
const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "") continue;
  if (!Object.hasOwn(fns, p[0])) {
    out.push("unknown function");
    continue;
  }
  const r = fns[p[0]].get(BigInt(p[1]));
  if (r.hit) hits++;
  else misses++;
  out.push(`${r.value} ${r.hit ? "hit" : "miss"}`);
}
out.push(`hits=${hits} misses=${misses}`);
console.log(out.join("\n"));
