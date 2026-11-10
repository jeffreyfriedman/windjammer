// The page side of the playground (ADR-0034): an editor, a Run button and two runners that can always be stopped.
//   Windjammer: the interpreter compiled to WebAssembly, in a Worker (worker.mjs) the page terminates on timeout.
//   TypeScript: types stripped and run inside <iframe sandbox="allow-scripts"> (ts-sandbox.html), in a Worker there.
// No code leaves the browser: a shared link carries the program in the URL fragment, which is never sent to a server.

const WALL_MS = 10_000; // a Windjammer run that outlives this is stopped by terminating its Worker
const TS_WALL_MS = 5_000; // the sandbox stops TypeScript runs itself; the page replaces the frame if it does not answer
const $ = (id) => document.getElementById(id);
const editor = $("source");
const langSel = $("lang");
const runBtn = $("run");
const stopBtn = $("stop");
const statusEl = $("status");
const stdoutEl = $("stdout");
const stderrEl = $("stderr");

const SAMPLES = {
  wj: editor.value,
  ts: [
    "interface Account {",
    "    owner: string;",
    "    balance: number;",
    "}",
    "",
    "function deposit(acct: Account, amount: number): void {",
    "    acct.balance += amount;",
    "}",
    "",
    "const acct: Account = { owner: \"Ada\", balance: 0 };",
    "for (const amount of [10, 20, 12]) {",
    "    deposit(acct, amount);",
    "}",
    "console.log(`${acct.owner} has ${acct.balance}`);",
    "",
  ].join("\n"),
};

let pending = null; // {id, timer, lang}
let nextId = 1;

function setStatus(text, kind) {
  statusEl.textContent = text;
  statusEl.dataset.kind = kind || "";
}

function finished() {
  if (pending) clearTimeout(pending.timer);
  pending = null;
  runBtn.disabled = false;
  stopBtn.disabled = true;
}

function show(r, ms) {
  stdoutEl.textContent = r.stdout;
  stderrEl.textContent = r.stderr;
  stderrEl.hidden = r.stderr === "";
  const what =
    r.exit_code === 0 ? "Finished"
    : (r.diagnostics || []).length > 0 && r.exit_code === 1 ? "The compiler found problems"
    : r.exit_code === 101 ? "Panicked"
    : r.exit_code === 124 ? "Stopped at a limit"
    : r.exit_code === 126 ? "Denied: the playground grants no files, network or environment"
    : `Exited with status ${r.exit_code}`;
  const steps = typeof r.steps === "number" ? `, ${r.steps.toLocaleString()} steps` : "";
  setStatus(`${what} (exit ${r.exit_code}${steps}, ${Math.round(ms)} ms)`, r.exit_code === 0 ? "ok" : "error");
}

// ---- Windjammer: wasm interpreter in a Worker ----
let worker = null;

function startWorker() {
  worker = new Worker(new URL("./worker.mjs", import.meta.url), { type: "module" });
  worker.onmessage = (ev) => {
    const m = ev.data;
    if ("ready" in m) {
      if (!pending && langSel.value === "wj") setStatus(m.ready ? "Ready." : `Could not load the interpreter: ${m.error}`, m.ready ? "" : "error");
      return;
    }
    if (!pending || m.id !== pending.id) return;
    finished();
    if (!m.ok) setStatus(`Could not run: ${m.error}`, "error");
    else show(m.result, m.ms);
  };
}

// ---- TypeScript: sandboxed iframe (opaque origin), Worker inside it ----
let frame = null;
let started = 0;

function startFrame() {
  if (frame) frame.remove();
  frame = document.createElement("iframe");
  frame.setAttribute("sandbox", "allow-scripts"); // never allow-same-origin: that would hand it this page's origin
  frame.hidden = true;
  frame.title = "TypeScript sandbox";
  frame.src = new URL("./ts-sandbox.html", import.meta.url).href;
  document.body.append(frame);
}

window.addEventListener("message", (ev) => {
  // only our sandbox frame, whose origin is opaque ("null"), may answer
  if (!frame || ev.source !== frame.contentWindow || ev.origin !== "null") return;
  const m = ev.data;
  if (!m || typeof m !== "object") return;
  if (m.type === "wj-ts-ready") {
    if (!pending && langSel.value === "ts") setStatus("Ready.", "");
    return;
  }
  if (m.type !== "wj-ts-result" || !pending || m.id !== pending.id) return;
  if (typeof m.stdout !== "string" || typeof m.stderr !== "string" || !Number.isInteger(m.exit_code)) return;
  finished();
  show({ stdout: m.stdout, stderr: m.stderr, exit_code: m.exit_code }, performance.now() - started);
});

// ---- shared ----
function stop(reason) {
  if (!pending) return;
  const lang = pending.lang;
  finished();
  if (lang === "wj") {
    worker.terminate();
    startWorker();
  } else {
    startFrame();
  }
  setStatus(reason, "error");
}

function runProgram() {
  if (pending) return;
  stdoutEl.textContent = "";
  stderrEl.textContent = "";
  stderrEl.hidden = true;
  const id = nextId++;
  const lang = langSel.value;
  const wall = lang === "wj" ? WALL_MS : TS_WALL_MS + 2_000;
  pending = { id, lang, timer: setTimeout(() => stop(`Stopped after ${wall / 1000} s.`), wall) };
  runBtn.disabled = true;
  stopBtn.disabled = false;
  setStatus("Running...", "");
  started = performance.now();
  if (lang === "wj") worker.postMessage({ id, source: editor.value });
  else frame.contentWindow.postMessage({ type: "wj-ts-run", id, source: editor.value, wallMs: TS_WALL_MS }, "*");
}

// #lang=ts&code=BASE64URL: a program shared as a link (decoded locally, never sent anywhere)
function fromFragment() {
  const lang = /(?:^#|&)lang=(wj|ts)\b/.exec(location.hash);
  const m = /(?:^#|&)code=([A-Za-z0-9_-]+)/.exec(location.hash);
  if (!m) return null;
  try {
    const b64 = m[1].replace(/-/g, "+").replace(/_/g, "/");
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    return { lang: lang ? lang[1] : "wj", code: new TextDecoder().decode(bytes) };
  } catch {
    return null;
  }
}

function toFragment(lang, text) {
  const bytes = new TextEncoder().encode(text);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return `#lang=${lang}&code=` + btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

const shared = fromFragment();
if (shared) {
  langSel.value = shared.lang;
  editor.value = shared.code;
}

langSel.addEventListener("change", () => {
  const other = langSel.value === "wj" ? "ts" : "wj";
  if (editor.value === SAMPLES[other]) editor.value = SAMPLES[langSel.value];
  setStatus("Ready.", "");
});
runBtn.addEventListener("click", runProgram);
stopBtn.addEventListener("click", () => stop("Stopped."));
$("share").addEventListener("click", () => {
  history.replaceState(null, "", toFragment(langSel.value, editor.value));
  setStatus("The link in the address bar now holds this program.", "");
});
editor.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    runProgram();
  }
  if (e.key === "Escape") editor.blur(); // Tab indents inside the editor; Escape gives keyboard focus back
  if (e.key === "Tab" && !e.shiftKey) {
    e.preventDefault();
    editor.setRangeText("    ", editor.selectionStart, editor.selectionEnd, "end");
  }
});

startWorker();
startFrame();
setStatus("Loading...", "");
