import * as fs from "node:fs";

const data = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0);
const t = parseInt(data[0], 10);
for (let i = 0; i < t; i++) {
  const s = BigInt(data[1 + i]).toString();
  let sum = 0;
  for (const c of s) sum += c.charCodeAt(0) - 48;
  let root = sum;
  while (root >= 10) {
    let r = 0;
    for (const c of String(root)) r += c.charCodeAt(0) - 48;
    root = r;
  }
  const rev = BigInt(s.split("").reverse().join(""));
  const rs = s.split("").reverse().join("");
  console.log(`${sum} ${root} ${rev} ${rs === s ? "yes" : "no"}`);
}
