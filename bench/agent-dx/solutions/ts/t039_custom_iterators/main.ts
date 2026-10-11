import * as fs from "node:fs";

function* fib(): Generator<bigint> {
  let a = 0n, b = 1n;
  for (;;) {
    yield a;
    [a, b] = [b, a + b];
  }
}
function* squares(): Generator<bigint> {
  for (let i = 1n;; i++) yield i * i;
}
function* primes(): Generator<bigint> {
  for (let n = 2n;; n++) {
    let p = true;
    for (let d = 2n; d * d <= n; d++) {
      if (n % d === 0n) {
        p = false;
        break;
      }
    }
    if (p) yield n;
  }
}
function* evens(): Generator<bigint> {
  for (let i = 0n;; i += 2n) yield i;
}
function* triangular(): Generator<bigint> {
  let s = 0n;
  for (let i = 1n;; i++) {
    s += i;
    yield s;
  }
}
const gens: Record<string, () => Generator<bigint>> = { fib, squares, primes, evens, triangular };

const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p.length < 3) continue;
  const n = BigInt(p[2]);
  const items: bigint[] = [];
  let sum = 0n;
  let count = 0n;
  for (const v of gens[p[0]]()) {
    if (p[1] === "below" ? v >= n : count >= n) break;
    items.push(v);
    sum += v;
    count++;
  }
  out.push(p[1] === "sum" ? String(sum) : items.join(" "));
}
if (out.length > 0) console.log(out.join("\n"));
