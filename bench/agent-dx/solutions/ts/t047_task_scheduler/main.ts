import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.trim()).filter((l) => l !== "");
const [W, N] = lines[0].split(/\s+/).map(Number);
type T = { id: number; dur: number; deps: number[]; start: number; end: number; started: boolean };
const tasks: T[] = [];
for (let i = 1; i <= N; i++) {
  const p = lines[i].split(/\s+/).map(Number);
  tasks.push({ id: p[0], dur: p[1], deps: p.slice(2), start: -1, end: -1, started: false });
}
tasks.sort((a, b) => a.id - b.id);
const byId = new Map(tasks.map((t) => [t.id, t]));
let time = 0;
let done = 0;
let running: T[] = [];
while (done < N) {
  const finished = running.filter((t) => t.end <= time);
  done += finished.length;
  running = running.filter((t) => t.end > time);
  for (const t of tasks) {
    if (running.length >= W) break;
    if (t.started) continue;
    if (t.deps.every((d) => byId.get(d)!.started && byId.get(d)!.end <= time)) {
      t.started = true;
      t.start = time;
      t.end = time + t.dur;
      running.push(t);
    }
  }
  if (done + 0 >= N && running.length === 0) break;
  if (running.length === 0) break;
  time = Math.min(...running.map((t) => t.end));
}
const out = tasks.map((t) => `${t.id} ${t.start} ${t.end}`);
out.push(`makespan ${tasks.reduce((m, t) => Math.max(m, t.end), 0)}`);
console.log(out.join("\n"));
