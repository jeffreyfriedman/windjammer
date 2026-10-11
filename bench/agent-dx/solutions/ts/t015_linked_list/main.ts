import * as fs from "node:fs";

let a: string[] = [];
const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  switch (p[0]) {
    case "push_front":
      a.unshift(String(BigInt(p[1])));
      break;
    case "push_back":
      a.push(String(BigInt(p[1])));
      break;
    case "pop_front":
      out.push(a.length ? a.shift()! : "empty");
      break;
    case "pop_back":
      out.push(a.length ? a.pop()! : "empty");
      break;
    case "remove": {
      const i = a.indexOf(String(BigInt(p[1])));
      if (i < 0) out.push("missing");
      else {
        a.splice(i, 1);
        out.push("removed");
      }
      break;
    }
    case "reverse":
      a = a.reverse();
      break;
    case "print":
      out.push(a.length ? a.join(" ") : "<empty>");
      break;
    case "nth": {
      const k = Number(p[1]);
      out.push(k >= 0 && k < a.length ? a[k] : "none");
      break;
    }
  }
}
if (out.length > 0) console.log(out.join("\n"));
