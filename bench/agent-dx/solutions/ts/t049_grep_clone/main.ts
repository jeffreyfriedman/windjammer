import * as fs from "node:fs";
import { Buffer } from "node:buffer";

let ic = false, num = false, cnt = false, inv = false;
const pos: string[] = [];
for (const a of process.argv.slice(2)) {
  if (a === "-i") ic = true;
  else if (a === "-n") num = true;
  else if (a === "-c") cnt = true;
  else if (a === "-v") inv = true;
  else pos.push(a);
}
const pattern = pos[0] ?? "";
const files = pos.slice(1);
const pat = ic ? pattern.toLowerCase() : pattern;
const multi = files.length > 1;
const out: string[] = [];
let total = 0;
let status = 0;
for (const f of files) {
  let text: string;
  try {
    text = fs.readFileSync(f, "latin1");
  } catch {
    out.push(`grep: ${f}: No such file`);
    status = 2;
    continue;
  }
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  let c = 0;
  lines.forEach((l, i) => {
    const m = (ic ? l.toLowerCase() : l).includes(pat);
    if (m === inv) return;
    c++;
    if (!cnt) out.push(`${multi ? f + ":" : ""}${num ? i + 1 + ":" : ""}${l}`);
  });
  total += c;
  if (cnt) out.push(`${multi ? f + ":" : ""}${c}`);
}
if (out.length > 0) fs.writeSync(1, Buffer.from(out.join("\n") + "\n", "latin1"));
process.exitCode = status !== 0 ? status : total > 0 ? 0 : 1;
