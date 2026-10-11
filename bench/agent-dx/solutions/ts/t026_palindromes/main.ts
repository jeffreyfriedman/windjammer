import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const out: string[] = [];
for (const raw of lines) {
  const s = raw.replace(/\r$/, "");
  if (s === "") {
    out.push("0 - 0");
    continue;
  }
  let best = 0, bestStart = 0, count = 0;
  const n = s.length;
  const expand = (l: number, r: number) => {
    while (l >= 0 && r < n && s[l] === s[r]) {
      count++;
      if (r - l + 1 > best || (r - l + 1 === best && l < bestStart)) {
        best = r - l + 1;
        bestStart = l;
      }
      l--;
      r++;
    }
  };
  for (let i = 0; i < n; i++) {
    expand(i, i);
    expand(i, i + 1);
  }
  out.push(`${best} ${s.slice(bestStart, bestStart + best)} ${count}`);
}
if (out.length > 0) console.log(out.join("\n"));
