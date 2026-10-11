import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.trim()).filter((l) => l !== "");
const n = parseInt(lines[0], 10);
const par = Array.from({ length: n }, (_, i) => i);
const sz = new Array<number>(n).fill(1);
let comps = n;
const find = (x: number): number => {
  while (par[x] !== x) {
    par[x] = par[par[x]];
    x = par[x];
  }
  return x;
};
const out: string[] = [];
for (const line of lines.slice(1)) {
  const p = line.split(/\s+/);
  if (p[0] === "union") {
    let a = find(Number(p[1])), b = find(Number(p[2]));
    if (a === b) out.push("same");
    else {
      if (sz[a] < sz[b]) [a, b] = [b, a];
      par[b] = a;
      sz[a] += sz[b];
      comps--;
      out.push("merged");
    }
  } else if (p[0] === "find") out.push(find(Number(p[1])) === find(Number(p[2])) ? "yes" : "no");
  else if (p[0] === "size") out.push(String(sz[find(Number(p[1]))]));
}
out.push(`components ${comps}`);
console.log(out.join("\n"));
