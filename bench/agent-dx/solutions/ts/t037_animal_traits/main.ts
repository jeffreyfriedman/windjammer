import * as fs from "node:fs";

interface Animal {
  name: string;
  sound: string;
  legs: number;
  describe(): string;
}

const base = (kind: string, sound: string, legs: number) => (name: string): Animal => ({
  name,
  sound,
  legs,
  describe() {
    return `${this.name} the ${kind} says ${this.sound} on ${this.legs} legs`;
  },
});

const kinds: Record<string, (name: string) => Animal> = {
  dog: base("dog", "woof", 4),
  cat: (name) => ({ ...base("cat", "meow", 4)(name), describe: () => `${name} the cat purrs softly` }),
  duck: base("duck", "quack", 2),
  snake: base("snake", "hiss", 0),
  spider: base("spider", "...", 8),
};

const out: string[] = [];
let legs = 0;
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "") continue;
  if (Object.hasOwn(kinds, p[0])) {
    const a = kinds[p[0]](p[1] ?? "");
    out.push(a.describe());
    legs += a.legs;
  } else out.push(`unknown kind ${p[0]}`);
}
out.push(`legs total=${legs}`);
console.log(out.join("\n"));
