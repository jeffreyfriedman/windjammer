import * as fs from "node:fs";

const tok = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const w = parseInt(tok[0], 10), n = parseInt(tok[1], 10);
const vals = tok.slice(2, 2 + n).map(Number);
const results = new Array<number>(n).fill(0);

const divisors = (v: number): number => {
  let c = 0;
  for (let d = 1; d * d <= v; d++) {
    if (v % d === 0) c += d * d === v ? 1 : 2;
  }
  return c;
};

let next = 0;
const worker = async (): Promise<void> => {
  while (next < n) {
    const i = next++;
    results[i] = divisors(vals[i]);
    await Promise.resolve();
  }
};
await Promise.all(Array.from({ length: w }, () => worker()));
console.log(vals.map((v, i) => `${v} ${results[i]}`).join("\n"));
