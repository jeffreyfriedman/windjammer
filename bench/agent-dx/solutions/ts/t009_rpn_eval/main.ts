import * as fs from "node:fs";

function evalRpn(line: string): string {
  const st: bigint[] = [];
  for (const tok of line.split(" ").filter((x) => x.length > 0)) {
    if (/^-?\d+$/.test(tok)) {
      st.push(BigInt(tok));
    } else if (tok === "+" || tok === "-" || tok === "*" || tok === "/") {
      if (st.length < 2) return "error: underflow";
      const b = st.pop()!, a = st.pop()!;
      if (tok === "+") st.push(a + b);
      else if (tok === "-") st.push(a - b);
      else if (tok === "*") st.push(a * b);
      else {
        if (b === 0n) return "error: divzero";
        st.push(a / b);
      }
    } else {
      return "error: token";
    }
  }
  return st.length === 1 ? String(st[0]) : "error: leftover";
}

for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const line = raw.replace(/\r$/, "");
  if (line.trim() === "") continue;
  console.log(evalRpn(line));
}
