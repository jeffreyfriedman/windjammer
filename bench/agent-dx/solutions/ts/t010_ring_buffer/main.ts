import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.trim()).filter((l) => l !== "");
const cap = parseInt(lines[0], 10);
const q: string[] = [];
const out: string[] = [];
for (const line of lines.slice(1)) {
  const p = line.split(/\s+/);
  switch (p[0]) {
    case "push":
      if (q.length >= cap) out.push("full");
      else {
        q.push(String(BigInt(p[1])));
        out.push("ok");
      }
      break;
    case "pop":
      out.push(q.length ? q.shift()! : "empty");
      break;
    case "peek":
      out.push(q.length ? q[0] : "empty");
      break;
    case "size":
      out.push(String(q.length));
      break;
    case "dump":
      out.push(q.length ? q.join(" ") : "<empty>");
      break;
  }
}
if (out.length > 0) console.log(out.join("\n"));
