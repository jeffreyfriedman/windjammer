import * as fs from "node:fs";

const words = new Set(fs.readFileSync(0, "utf8").toLowerCase().split(/\s+/).filter((x) => x.length > 0));
const groups = new Map<string, string[]>();
for (const w of words) {
  const k = w.split("").sort().join("");
  const g = groups.get(k);
  if (g) g.push(w);
  else groups.set(k, [w]);
}
const lines: string[][] = [];
for (const g of groups.values()) {
  if (g.length >= 2) lines.push(g.sort());
}
lines.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
console.log(lines.length ? lines.map((l) => l.join(" ")).join("\n") : "none");
