import * as fs from "node:fs";

class Err extends Error {}

function evaluate(src: string): bigint {
  let pos = 0;
  const skip = () => {
    while (src[pos] === " " || src[pos] === "\t") pos++;
  };
  const expr = (): bigint => {
    let v = term();
    for (;;) {
      skip();
      const c = src[pos];
      if (c === "+") {
        pos++;
        v += term();
      } else if (c === "-") {
        pos++;
        v -= term();
      } else return v;
    }
  };
  const term = (): bigint => {
    let v = unary();
    for (;;) {
      skip();
      const c = src[pos];
      if (c === "*" || c === "/" || c === "%") {
        pos++;
        const r = unary();
        if (c === "*") v *= r;
        else {
          if (r === 0n) throw new Err("div0");
          v = c === "/" ? v / r : v % r;
        }
      } else return v;
    }
  };
  const unary = (): bigint => {
    skip();
    if (src[pos] === "-") {
      pos++;
      return -unary();
    }
    if (src[pos] === "(") {
      pos++;
      const v = expr();
      skip();
      if (src[pos] !== ")") throw new Err("paren");
      pos++;
      return v;
    }
    const m = /^\d+/.exec(src.slice(pos));
    if (!m) throw new Err("operand");
    pos += m[0].length;
    return BigInt(m[0]);
  };
  const v = expr();
  skip();
  if (pos !== src.length) throw new Err("trailing");
  return v;
}

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const out = lines.map((l) => {
  try {
    return String(evaluate(l.replace(/\r$/, "")));
  } catch (e) {
    if (e instanceof Err) return "error";
    throw e;
  }
});
if (out.length > 0) console.log(out.join("\n"));
