import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
let ok = 0, invalid = 0, overflow = 0, empty = 0;
const out: string[] = [];
for (const raw of lines) {
  const t = raw.replace(/\r$/, "");
  if (t === "") {
    empty++;
    out.push("error empty");
  } else if (!/^[-+]?[0-9]+$/.test(t)) {
    invalid++;
    out.push(`error invalid ${t}`);
  } else {
    const v = BigInt(t);
    if (v < -2147483648n || v > 2147483647n) {
      overflow++;
      out.push(`error overflow ${t}`);
    } else {
      ok++;
      out.push(`ok ${v}`);
    }
  }
}
out.push(`summary ok=${ok} invalid=${invalid} overflow=${overflow} empty=${empty}`);
console.log(out.join("\n"));
