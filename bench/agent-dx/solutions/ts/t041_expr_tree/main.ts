import * as fs from "node:fs";

type Expr =
  | { kind: "lit"; v: bigint }
  | { kind: "neg"; a: Expr }
  | { kind: "bin"; op: string; a: Expr; b: Expr };

function parse(src: string): Expr {
  const toks = src.replace(/\(/g, "( ").replace(/\)/g, " )").split(/\s+/).filter((x) => x !== "");
  let i = 0;
  const go = (): Expr => {
    const t = toks[i++];
    if (t !== "(") return { kind: "lit", v: BigInt(t) };
    const op = toks[i++];
    let e: Expr;
    if (op === "neg") e = { kind: "neg", a: go() };
    else {
      const a = go();
      const b = go();
      e = { kind: "bin", op, a, b };
    }
    i++;
    return e;
  };
  return go();
}

const evalE = (e: Expr): bigint =>
  e.kind === "lit" ? e.v : e.kind === "neg" ? -evalE(e.a) : e.op === "+" ? evalE(e.a) + evalE(e.b) : e.op === "-" ? evalE(e.a) - evalE(e.b) : evalE(e.a) * evalE(e.b);
const depth = (e: Expr): number =>
  e.kind === "lit" ? 1 : e.kind === "neg" ? 1 + depth(e.a) : 1 + Math.max(depth(e.a), depth(e.b));
const nodes = (e: Expr): number => (e.kind === "lit" ? 1 : e.kind === "neg" ? 1 + nodes(e.a) : 1 + nodes(e.a) + nodes(e.b));
const show = (e: Expr): string =>
  e.kind === "lit" ? String(e.v) : e.kind === "neg" ? `-(${show(e.a)})` : `(${show(e.a)} ${e.op} ${show(e.b)})`;

const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const l = raw.trim();
  if (l === "") continue;
  const e = parse(l);
  out.push(`${evalE(e)} ${depth(e)} ${nodes(e)} ${show(e)}`);
}
if (out.length > 0) console.log(out.join("\n"));
