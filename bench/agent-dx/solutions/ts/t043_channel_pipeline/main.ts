import * as fs from "node:fs";

type Stage = (src: AsyncIterable<bigint>) => AsyncIterable<bigint>;

async function* source(n: number): AsyncGenerator<bigint> {
  for (let i = 1; i <= n; i++) yield BigInt(i);
}

function mkStage(tok: string): Stage {
  const [name, arg] = tok.split(":");
  const k = arg === undefined ? 0n : BigInt(arg);
  return async function* (src) {
    let p1: bigint | null = null, p2: bigint | null = null;
    for await (const x of src) {
      switch (name) {
        case "add": yield x + k; break;
        case "mul": yield x * k; break;
        case "gt": if (x > k) yield x; break;
        case "even": if (x % 2n === 0n) yield x; break;
        case "odd": if (x % 2n !== 0n) yield x; break;
        case "dedupe3":
          if (x !== p1 && x !== p2) yield x;
          p2 = p1;
          p1 = x;
          break;
      }
    }
  };
}

const lines = fs.readFileSync(0, "utf8").split("\n");
const n = parseInt(lines[0], 10);
const stages = (lines[1] ?? "").trim().split(/\s+/).filter((x) => x !== "").map(mkStage);
let stream: AsyncIterable<bigint> = source(n);
for (const s of stages) stream = s(stream);
const items: bigint[] = [];
let sum = 0n;
for await (const x of stream) {
  items.push(x);
  sum += x;
}
console.log(`${items.length}\n${items.join(" ")}\nsum=${sum}`);
