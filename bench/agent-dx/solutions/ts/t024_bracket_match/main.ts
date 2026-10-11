import * as fs from "node:fs";

const pairs: Record<string, string> = { ")": "(", "]": "[", "}": "{" };

function check(line: string): string {
  const st: number[] = [];
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === "(" || c === "[" || c === "{") st.push(i);
    else if (c in pairs) {
      if (st.length === 0 || line[st[st.length - 1]] !== pairs[c]) return `error ${i + 1}`;
      st.pop();
    }
  }
  return st.length ? `error ${st[st.length - 1] + 1}` : "ok";
}

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const out = lines.map((l) => check(l.replace(/\r$/, "")));
if (out.length > 0) console.log(out.join("\n"));
