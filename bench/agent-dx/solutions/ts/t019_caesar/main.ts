import * as fs from "node:fs";

const s = fs.readFileSync(0, "utf8");
const nl = s.indexOf("\n");
const head = (nl < 0 ? s : s.slice(0, nl)).trim().split(/\s+/);
const text = nl < 0 ? "" : s.slice(nl + 1);
let n = parseInt(head[1], 10);
if (head[0] === "dec") n = -n;
n = ((n % 26) + 26) % 26;
const res = text.replace(/[A-Za-z]/g, (c) => {
  const base = c <= "Z" ? 65 : 97;
  return String.fromCharCode(((c.charCodeAt(0) - base + n) % 26) + base);
});
fs.writeSync(1, res);
