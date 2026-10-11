import * as fs from "node:fs";

const tok = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const w = parseInt(tok[0], 10), n = parseInt(tok[1], 10);
const vals = tok.slice(2, 2 + n).map(Number);
const hist = new Array<number>(10).fill(0);
const counts = new Array<number>(w).fill(0);

const worker = async (id: number): Promise<void> => {
  for (let i = id; i < n; i += w) {
    hist[vals[i]]++;
    counts[id]++;
  }
};
await Promise.all(Array.from({ length: w }, (_, i) => worker(i)));
const out = [`hist: ${hist.join(" ")}`];
counts.forEach((c, i) => out.push(`worker ${i}: ${c}`));
out.push(`checksum: ${hist.reduce((a, c, d) => a + c * d, 0)}`);
console.log(out.join("\n"));
