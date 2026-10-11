import process from "node:process";

const args = process.argv.slice(2);
let name = "anonymous", count = 1n, verb = 0, dry = false;
const posArgs: string[] = [];

const fail = (msg: string): never => {
  console.log(msg);
  process.exit(2);
};
const setCount = (v: string) => {
  if (!/^[0-9]+$/.test(v)) fail(`error: invalid count ${v}`);
  count = BigInt(v);
};

for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a === "--") {
    posArgs.push(...args.slice(i + 1));
    break;
  } else if (a.startsWith("--name=")) name = a.slice(7);
  else if (a === "--name") {
    if (i + 1 >= args.length) fail("error: missing value for --name");
    name = args[++i];
  } else if (a.startsWith("--count=")) setCount(a.slice(8));
  else if (a === "--count") {
    if (i + 1 >= args.length) fail("error: missing value for --count");
    setCount(args[++i]);
  } else if (a === "--dry-run") dry = true;
  else if (/^-v+$/.test(a)) verb += a.length - 1;
  else if (a.startsWith("-") && a !== "-") fail(`error: unknown option ${a}`);
  else posArgs.push(a);
}
console.log(`name: ${name}\ncount: ${count}\nverbosity: ${verb}\ndry-run: ${dry}\npositional: ${posArgs.join(" ")}`);
