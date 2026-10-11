import * as fs from "node:fs";

const tok = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const w = parseInt(tok[0], 10), n = parseInt(tok[1], 10);
const vals = tok.slice(2, 2 + n).map(BigInt);
const q = Math.floor(n / w), r = n % w;

const worker = async (chunk: bigint[]): Promise<bigint> => {
  let s = 0n;
  for (const v of chunk) s += v;
  return s;
};

const jobs: Promise<bigint>[] = [];
let start = 0;
for (let i = 0; i < w; i++) {
  const len = q + (i < r ? 1 : 0);
  jobs.push(worker(vals.slice(start, start + len)));
  start += len;
}
const sums = await Promise.all(jobs);
const out = sums.map((s, i) => `chunk ${i}: ${s}`);
out.push(`total: ${sums.reduce((a, b) => a + b, 0n)}`);
console.log(out.join("\n"));
