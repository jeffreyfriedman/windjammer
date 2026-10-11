import * as fs from "node:fs";

const tok = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0).map(Number);
let pos = 0;
const readMat = (r: number, c: number): number[][] =>
  Array.from({ length: r }, () => Array.from({ length: c }, () => tok[pos++]));
const k = tok[pos++];
const out: string[] = [];
for (let t = 0; t < k; t++) {
  const r1 = tok[pos++], c1 = tok[pos++];
  const A = readMat(r1, c1);
  const r2 = tok[pos++], c2 = tok[pos++];
  const B = readMat(r2, c2);
  if (c1 !== r2) out.push("dimension mismatch");
  else {
    for (let i = 0; i < r1; i++) {
      const row: number[] = [];
      for (let j = 0; j < c2; j++) {
        let s = 0;
        for (let x = 0; x < c1; x++) s += A[i][x] * B[x][j];
        row.push(s);
      }
      out.push(row.join(" "));
    }
  }
  out.push("---");
}
if (out.length > 0) console.log(out.join("\n"));
