import * as fs from "node:fs";

function enc(s: string): string {
  let out = "";
  for (let i = 0; i < s.length;) {
    let j = i;
    while (j < s.length && s[j] === s[i]) j++;
    out += `${j - i}${s[i]}`;
    i = j;
  }
  return out;
}

function dec(s: string): string {
  let out = "";
  for (const m of s.matchAll(/(\d+)([a-z])/g)) out += m[2].repeat(parseInt(m[1], 10));
  return out;
}

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const out: string[] = [];
for (const raw of lines) {
  const line = raw.replace(/\r$/, "");
  if (line === "") continue;
  const text = line.slice(2);
  out.push(line[0] === "E" ? enc(text) : dec(text));
}
if (out.length > 0) console.log(out.join("\n"));
