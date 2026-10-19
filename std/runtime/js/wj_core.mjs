// Windjammer JavaScript runtime core (ES2022, Node >= 20 or modern browsers).
//
// Representation contract (see src/codegen/javascript/MODULE.md):
//   i64/u64/isize/usize -> BigInt     i8..i32/u8..u32 -> Number (range checked)
//   f64 -> Number    f32 -> Math.fround(Number)    char -> 1-code-point string
//   Vec -> Array     HashMap -> Map    HashSet -> Set    tuple -> Array
//   Option -> null | {v}     Result -> {ok: true|false, v}
// Arithmetic that overflows panics (like Rust debug builds); `wrapping_*` ops wrap explicitly.

const isNode = typeof process !== "undefined" && process.versions && process.versions.node;
const fs = isNode ? await import("node:fs") : null;
const threads = isNode ? await import("node:worker_threads") : null;
/** Stack of the thread that runs `main` (Rust's main thread has 8 MiB; Node's default is ~1 MiB). */
const MAIN_STACK_MB = 64;

export class WjPanic extends Error {}

export function $panic(msg) {
  throw new WjPanic(String(msg));
}

// ---------------------------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------------------------
let outBuf = [];
let outLen = 0;

function writeAll(fd, text) {
  const data = Buffer.from(text);
  let off = 0;
  while (off < data.length) {
    try {
      off += fs.writeSync(fd, data, off);
    } catch (e) {
      if (e.code === "EAGAIN") continue;
      if (e.code === "EPIPE") return;
      throw e;
    }
  }
}

export function $flush() {
  if (outBuf.length === 0) return;
  const text = outBuf.join("");
  outBuf = [];
  outLen = 0;
  if (fs) writeAll(1, text);
  else console.log(text);
}

export function $print(s) {
  outBuf.push(s);
  outLen += s.length;
  if (outLen > 65536) $flush();
}

export function $eprint(s) {
  $flush();
  if (fs) writeAll(2, s);
  else console.error(s);
}

/**
 * Run `main` with Rust's process semantics (panic = exit 101, buffered stdout). Under Node the
 * program runs on a worker thread with a Rust-like stack, so recursion as deep as a Rust program
 * allows does not overflow Node's small default stack; `entry` is the program module's URL.
 */
export function $run(main, entry) {
  if (threads && threads.isMainThread && entry) {
    const w = new threads.Worker(new URL(entry), { resourceLimits: { stackSizeMb: MAIN_STACK_MB } });
    w.on("error", (e) => {
      writeAll(2, `internal error: ${(e && e.stack) || e}\n`);
      process.exitCode = 70;
    });
    w.on("exit", (code) => {
      if (code !== 0 || process.exitCode === undefined) process.exitCode = code;
    });
    return;
  }
  try {
    main();
    $flush();
  } catch (e) {
    $flush();
    if (e instanceof WjPanic) {
      $eprint(`thread 'main' panicked:\n${e.message}\n`);
      if (typeof process !== "undefined") process.exitCode = 101;
    } else if (e instanceof RangeError && /call stack/i.test(e.message)) {
      $eprint("thread 'main' has overflowed its stack\n");
      if (typeof process !== "undefined") process.exitCode = 134;
    } else {
      $eprint(`internal error: ${(e && e.stack) || e}\n`);
      if (typeof process !== "undefined") process.exitCode = 70;
    }
  }
}

// ---------------------------------------------------------------------------------------------
// Checked integer ranges
// ---------------------------------------------------------------------------------------------
const ovf = () => $panic("attempt to perform arithmetic with overflow");
export const $i8 = (r) => (r >= -128 && r <= 127 ? r : ovf());
export const $i16 = (r) => (r >= -32768 && r <= 32767 ? r : ovf());
export const $i32 = (r) => (r >= -2147483648 && r <= 2147483647 ? r : ovf());
export const $u8 = (r) => (r >= 0 && r <= 255 ? r : ovf());
export const $u16 = (r) => (r >= 0 && r <= 65535 ? r : ovf());
export const $u32 = (r) => (r >= 0 && r <= 4294967295 ? r : ovf());
export const $i64 = (r) => (r >= -9223372036854775808n && r <= 9223372036854775807n ? r : ovf());
export const $u64 = (r) => (r >= 0n && r <= 18446744073709551615n ? r : ovf());

export const $divBig = (a, b) => (b === 0n ? $panic("attempt to divide by zero") : a / b);
export const $remBig = (a, b) => (b === 0n ? $panic("attempt to calculate the remainder with a divisor of zero") : a % b);
export const $divNum = (a, b) => (b === 0 ? $panic("attempt to divide by zero") : Math.trunc(a / b));
export const $remNum = (a, b) => (b === 0 ? $panic("attempt to calculate the remainder with a divisor of zero") : a % b);
export const $shc = (n, bits) => (n >= 0 && n < bits ? n : $panic("attempt to shift with overflow"));

export function $f2i(x, lo, hi, wide) {
  if (Number.isNaN(x)) return wide ? 0n : 0;
  const t = Math.trunc(x);
  let r;
  if (t <= Number(lo)) r = lo;
  else if (t >= Number(hi)) r = hi;
  else r = BigInt(t);
  return wide ? r : Number(r);
}

export const $min2 = (a, b) => (a < b ? a : b);
export const $max2 = (a, b) => (a > b ? a : b);
export const $sat = (lo, hi, r) => (r < lo ? lo : r > hi ? hi : r);
export const $chkd = (lo, hi, r) => (r < lo || r > hi ? null : { v: r });

export function $remEuclid(a, b) {
  const zero = typeof a === "bigint" ? 0n : 0;
  if (b === zero) $panic("attempt to calculate the remainder with a divisor of zero");
  const r = a % b;
  return r < zero ? (b < zero ? r - b : r + b) : r;
}

export function $divEuclid(a, b) {
  const zero = typeof a === "bigint" ? 0n : 0;
  if (b === zero) $panic("attempt to divide by zero");
  const q = typeof a === "bigint" ? a / b : Math.trunc(a / b);
  const r = a % b;
  return r < zero ? (b > zero ? q - (typeof a === "bigint" ? 1n : 1) : q + (typeof a === "bigint" ? 1n : 1)) : q;
}

export function $popcnt(v, bits) {
  let x = BigInt.asUintN(bits, BigInt(v));
  let c = 0;
  while (x) {
    c += Number(x & 1n);
    x >>= 1n;
  }
  return typeof v === "bigint" ? BigInt(c) : c >>> 0;
}

export function $clz(v, bits) {
  const x = BigInt.asUintN(bits, BigInt(v));
  const n = x === 0n ? bits : bits - x.toString(2).length;
  return typeof v === "bigint" ? BigInt(n) : n;
}

export function $ctz(v, bits) {
  let x = BigInt.asUintN(bits, BigInt(v));
  if (x === 0n) return typeof v === "bigint" ? BigInt(bits) : bits;
  let n = 0;
  while ((x & 1n) === 0n) {
    x >>= 1n;
    n++;
  }
  return typeof v === "bigint" ? BigInt(n) : n;
}

function rot(v, n, bits, signed, left) {
  const b = BigInt(bits);
  let x = BigInt.asUintN(bits, BigInt(v));
  const k = BigInt(Number(n) % bits);
  const r = left ? ((x << k) | (x >> (b - k))) : ((x >> k) | (x << (b - k)));
  const out = signed ? BigInt.asIntN(bits, r) : BigInt.asUintN(bits, r);
  return typeof v === "bigint" ? out : Number(out);
}
export const $rotl = (v, n, bits, signed) => rot(v, n, bits, signed, true);
export const $rotr = (v, n, bits, signed) => rot(v, n, bits, signed, false);

export const $round = (x) => (x < 0 ? -Math.round(-x) : Math.round(x));
export const $fmin = (a, b) => (Number.isNaN(a) ? b : Number.isNaN(b) ? a : Math.min(a, b));
export const $fmax = (a, b) => (Number.isNaN(a) ? b : Number.isNaN(b) ? a : Math.max(a, b));
export const $fsignum = (x) => (Number.isNaN(x) ? NaN : $signNeg(x) ? -1 : 1);
export const $signNeg = (x) => x < 0 || Object.is(x, -0);

// ---------------------------------------------------------------------------------------------
// Equality, ordering, cloning
// ---------------------------------------------------------------------------------------------
export function $eq(a, b) {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((x, i) => $eq(x, b[i]));
  if (a instanceof Map) {
    if (!(b instanceof Map) || a.size !== b.size) return false;
    for (const [k, v] of a) if (!b.has(k) || !$eq(v, b.get(k))) return false;
    return true;
  }
  if (a instanceof Set) {
    if (!(b instanceof Set) || a.size !== b.size) return false;
    for (const k of a) if (!b.has(k)) return false;
    return true;
  }
  const ka = Object.keys(a);
  if (ka.length !== Object.keys(b).length) return false;
  return ka.every((k) => $eq(a[k], b[k]));
}

export function $cmp(a, b) {
  if (typeof a === "bigint" || typeof a === "number" || typeof a === "boolean") return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === "string") return cmpStr(a, b);
  if (a === null || b === null) return a === b ? 0 : a === null ? -1 : 1;
  if (Array.isArray(a)) {
    for (let i = 0; i < Math.min(a.length, b.length); i++) {
      const c = $cmp(a[i], b[i]);
      if (c !== 0) return c;
    }
    return a.length - b.length < 0 ? -1 : a.length > b.length ? 1 : 0;
  }
  for (const k of Object.keys(a)) {
    if (k === "$t") continue;
    const c = $cmp(a[k], b[k]);
    if (c !== 0) return c;
  }
  return 0;
}

// `Ordering` is -1 / 0 / 1.
export const $dbgOrd = (x) => (x < 0 ? "Less" : x > 0 ? "Greater" : "Equal");
const hasNaN = (x) => (typeof x === "number" && Number.isNaN(x)) || (Array.isArray(x) && x.some(hasNaN));
/** `partial_cmp`: None when a float compared is NaN. */
export const $pcmp = (a, b) => (hasNaN(a) || hasNaN(b) ? null : { v: $cmp(a, b) });
/** Stable in-place sort by a comparator returning an `Ordering` (Rust `sort_by` is stable). */
export function $sortBy(v, f) {
  v.sort((x, y) => f(x, y));
}
/** `max_by` (last maximum, dir 1) / `min_by` (first minimum, dir -1) over a comparator. */
export function $minmaxBy(v, f, dir) {
  if (v.length === 0) return null;
  let best = v[0];
  for (let i = 1; i < v.length; i++) {
    const c = f(v[i], best);
    if (dir < 0 ? c < 0 : c >= 0) best = v[i];
  }
  return { v: best };
}

function cmpStr(a, b) {
  const x = Array.from(a);
  const y = Array.from(b);
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const p = x[i].codePointAt(0);
    const q = y[i].codePointAt(0);
    if (p !== q) return p < q ? -1 : 1;
  }
  return x.length < y.length ? -1 : x.length > y.length ? 1 : 0;
}

export function $cl(x) {
  if (x === null || typeof x !== "object") return x;
  if (Array.isArray(x)) return x.map($cl);
  if (x instanceof Map) return new Map(Array.from(x, ([k, v]) => [k, $cl(v)]));
  if (x instanceof Set) return new Set(x);
  if (typeof x.clone === "function") return x.clone();
  const o = {};
  for (const k of Object.keys(x)) o[k] = $cl(x[k]);
  return o;
}

export function $rep(x, n, c) {
  const len = Number(n);
  if (c === null) return new Array(len).fill(x);
  return Array.from({ length: len }, (_, i) => (i === 0 ? x : c(x)));
}

// ---------------------------------------------------------------------------------------------
// Vec / slices
// ---------------------------------------------------------------------------------------------
export function $ix(v, i) {
  const n = Number(i);
  if (!(n >= 0 && n < v.length)) $panic(`index out of bounds: the len is ${v.length} but the index is ${i}`);
  return n;
}
export const $at = (v, i) => v[$ix(v, i)];

// ---------------------------------------------------------------------------------------------
// `&mut` of a value-represented place (numbers, strings, Option, Result, enums): an accessor
// cell `{ v }` whose reads and writes go to the place. The container and key are evaluated once,
// when the borrow is taken (Rust evaluates the place expression at the borrow).
// ---------------------------------------------------------------------------------------------
export function $cellOf(o, k) {
  return { get v() { return o[k]; }, set v(x) { o[k] = x; } };
}
export const $cellIx = (v, i) => $cellOf(v, $ix(v, i));
/** `*r = x` through an alias of an object-represented value (struct, Vec, HashMap, HashSet, tuple). */
export function $replace(dst, src) {
  if (dst === src) return;
  if (Array.isArray(dst)) { dst.length = 0; for (const x of src) dst.push(x); return; }
  if (dst instanceof Map) { dst.clear(); for (const [k, v] of src) dst.set(k, v); return; }
  if (dst instanceof Set) { dst.clear(); for (const x of src) dst.add(x); return; }
  Object.assign(dst, src);
}
export const $pop = (v) => (v.length ? { v: v.pop() } : null);
export const $popFront = (v) => (v.length ? { v: v.shift() } : null);
export const $vget = (v, i) => {
  const n = Number(i);
  return n >= 0 && n < v.length ? { v: v[n] } : null;
};
export const $vlast = (v) => (v.length ? { v: v[v.length - 1] } : null);
export function $vinsert(v, i, x) {
  const n = Number(i);
  if (n > v.length) $panic(`insertion index (is ${i}) should be <= len (is ${v.length})`);
  v.splice(n, 0, x);
}
export function $vremove(v, i) {
  const n = $ix(v, i);
  return v.splice(n, 1)[0];
}
export function $vswapRemove(v, i) {
  const n = $ix(v, i);
  const x = v[n];
  v[n] = v[v.length - 1];
  v.pop();
  return x;
}
export function $sort(v) {
  v.sort($cmp);
}
export function $dedup(v) {
  let w = 0;
  for (let r = 0; r < v.length; r++) if (w === 0 || !$eq(v[w - 1], v[r])) v[w++] = v[r];
  v.length = w;
}
export function $extend(v, o) {
  for (const x of o) v.push(x);
}
export function $truncate(v, n) {
  const k = Number(n);
  if (k < v.length) v.length = k;
}
export function $swap(v, i, j) {
  const a = $ix(v, i);
  const b = $ix(v, j);
  [v[a], v[b]] = [v[b], v[a]];
}
export function $drain(v) {
  return v.splice(0, v.length);
}
export function $windows(v, n) {
  const k = Number(n);
  const out = [];
  for (let i = 0; i + k <= v.length; i++) out.push(v.slice(i, i + k));
  return out;
}
export function $chunks(v, n) {
  const k = Number(n);
  if (k === 0) $panic("chunk size must be non-zero");
  const out = [];
  for (let i = 0; i < v.length; i += k) out.push(v.slice(i, i + k));
  return out;
}
export function $bsearch(v, x) {
  let lo = 0;
  let hi = v.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    const c = $cmp(v[mid], x);
    if (c === 0) return { ok: true, v: BigInt(mid) };
    if (c < 0) lo = mid + 1;
    else hi = mid;
  }
  return { ok: false, v: BigInt(lo) };
}
export function $retain(v, f) {
  let w = 0;
  for (let r = 0; r < v.length; r++) if (f(v[r])) v[w++] = v[r];
  v.length = w;
}
export function $splitOff(v, n) {
  const k = Number(n);
  if (k > v.length) $panic("`at` split index out of bounds");
  return v.splice(k);
}
export function $splitAt(v, n) {
  const k = Number(n);
  if (k > v.length) $panic("mid > len");
  return [v.slice(0, k), v.slice(k)];
}
export function $vecSlice(v, a, b, incl) {
  const s = Number(a);
  const e = Number(b) + (incl ? 1 : 0);
  if (s > e) $panic(`slice index starts at ${s} but ends at ${e}`);
  if (e > v.length) $panic(`range end index ${e} out of range for slice of length ${v.length}`);
  return v.slice(s, e);
}
export function $range(a, b, incl) {
  const out = [];
  if (typeof a === "bigint") {
    for (let i = a; incl ? i <= b : i < b; i++) out.push(i);
  } else {
    for (let i = a; incl ? i <= b : i < b; i++) out.push(i);
  }
  return out;
}
export const $zip = (a, b) => {
  const n = Math.min(a.length, b.length);
  const out = new Array(n);
  for (let i = 0; i < n; i++) out[i] = [a[i], b[i]];
  return out;
};
export function $filterMap(v, f) {
  const out = [];
  for (const x of v) {
    const r = f(x);
    if (r !== null) out.push(r.v);
  }
  return out;
}
export function $takeWhile(v, f) {
  const out = [];
  for (const x of v) {
    if (!f(x)) break;
    out.push(x);
  }
  return out;
}
export function $skipWhile(v, f) {
  let i = 0;
  while (i < v.length && f(v[i])) i++;
  return v.slice(i);
}
export function $collectResult(v) {
  const out = [];
  for (const r of v) {
    if (!r.ok) return r;
    out.push(r.v);
  }
  return { ok: true, v: out };
}
export function $collectOption(v) {
  const out = [];
  for (const o of v) {
    if (o === null) return null;
    out.push(o.v);
  }
  return { v: out };
}
export function $minmax(v, dir) {
  if (v.length === 0) return null;
  let best = v[0];
  for (let i = 1; i < v.length; i++) {
    const c = $cmp(v[i], best);
    if (dir < 0 ? c < 0 : c >= 0) best = v[i];
  }
  return { v: best };
}
export function $find(v, f) {
  for (const x of v) if (f(x)) return { v: x };
  return null;
}
export function $position(v, f) {
  for (let i = 0; i < v.length; i++) if (f(v[i])) return { v: BigInt(i) };
  return null;
}

// ---------------------------------------------------------------------------------------------
// Option / Result
// ---------------------------------------------------------------------------------------------
export const $unwrapOpt = (o) => (o === null ? $panic("called `Option::unwrap()` on a `None` value") : o.v);
export const $expectOpt = (o, m) => (o === null ? $panic(m) : o.v);
export const $unwrapOr = (o, d) => (o === null ? d : o.v);
export const $optOrElse = (o, f) => (o === null ? f() : o.v);
export const $optMap = (o, f) => (o === null ? null : { v: f(o.v) });
export const $optAndThen = (o, f) => (o === null ? null : f(o.v));
export const $optFilter = (o, f) => (o !== null && f(o.v) ? o : null);
export const $okOr = (o, e) => (o === null ? { ok: false, v: e } : { ok: true, v: o.v });
export const $optMapOr = (o, d, f) => (o === null ? d : f(o.v));
export const $unwrapRes = (r, dbg) => (r.ok ? r.v : $panic(`called \`Result::unwrap()\` on an \`Err\` value: ${dbg(r.v)}`));
export const $expectRes = (r, m, dbg) => (r.ok ? r.v : $panic(`${m}: ${dbg(r.v)}`));
export const $unwrapErr = (r) => (r.ok ? $panic("called `Result::unwrap_err()` on an `Ok` value") : r.v);
export const $resOrElse = (r, f) => (r.ok ? r.v : f(r.v));
export const $resMap = (r, f) => (r.ok ? { ok: true, v: f(r.v) } : r);
export const $resMapErr = (r, f) => (r.ok ? r : { ok: false, v: f(r.v) });
export const $resAndThen = (r, f) => (r.ok ? f(r.v) : r);
export const $resOk = (r) => (r.ok ? { v: r.v } : null);
export const $resErr = (r) => (r.ok ? null : { v: r.v });

// ---------------------------------------------------------------------------------------------
// HashMap / HashSet
// ---------------------------------------------------------------------------------------------
export function $mapInsert(m, k, v) {
  const old = m.has(k) ? { v: m.get(k) } : null;
  m.set(k, v);
  return old;
}
export const $mapGetOpt = (m, k) => (m.has(k) ? { v: m.get(k) } : null);
export const $mget = (m, k) => (m.has(k) ? m.get(k) : $panic("key not found in map"));
export function $mapRemove(m, k) {
  if (!m.has(k)) return null;
  const v = m.get(k);
  m.delete(k);
  return { v };
}
export function $setInsert(s, x) {
  if (s.has(x)) return false;
  s.add(x);
  return true;
}

// ---------------------------------------------------------------------------------------------
// Strings (UTF-8 byte semantics for len/slicing/find)
// ---------------------------------------------------------------------------------------------
const encoder = new TextEncoder();
const strictDecoder = new TextDecoder("utf-8", { fatal: true });

export function $strlen(s) {
  let n = 0;
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c < 0x80) n += 1;
    else if (c < 0x800) n += 2;
    else if (c >= 0xd800 && c <= 0xdbff) {
      n += 4;
      i++;
    } else n += 3;
  }
  return n;
}

export function $strSlice(s, a, b, incl) {
  const bytes = encoder.encode(s);
  const start = Number(a);
  const end = Number(b) + (incl ? 1 : 0);
  if (start > end) $panic(`begin <= end (${start} <= ${end}) when slicing \`${s}\``);
  if (end > bytes.length) $panic(`byte index ${end} is out of bounds of \`${s}\``);
  try {
    return strictDecoder.decode(bytes.subarray(start, end));
  } catch {
    return $panic(`byte index is not a char boundary; it is inside a character of \`${s}\``);
  }
}

export function $split(s, sep) {
  if (sep === "") return ["", ...Array.from(s), ""];
  return s.split(sep);
}
export const $bytes = (s) => Array.from(encoder.encode(s));
export function $lines(s) {
  if (s === "") return [];
  const parts = s.split("\n").map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l));
  if (parts[parts.length - 1] === "") parts.pop();
  return parts;
}
export function $charIndices(s) {
  const out = [];
  let off = 0;
  for (const c of s) {
    out.push([BigInt(off), c]);
    off += $strlen(c);
  }
  return out;
}
export function $strFind(s, pat) {
  const i = s.indexOf(pat);
  return i < 0 ? null : { v: BigInt($strlen(s.slice(0, i))) };
}
export const $asciiUpper = (c) => (c >= "a" && c <= "z" ? c.toUpperCase() : c);
export const $asciiLower = (c) => (c >= "A" && c <= "Z" ? c.toLowerCase() : c);
export function $toDigit(c, radix) {
  const r = Number(radix);
  const d = parseInt(c, 36);
  if (c.length !== 1 || Number.isNaN(d) || d >= r) return null;
  return { v: d };
}
export const $charFromU32 = (n) => {
  const x = Number(n);
  return x > 0x10ffff || (x >= 0xd800 && x <= 0xdfff) ? null : { v: String.fromCodePoint(x) };
};

export function $parseInt(s, lo, hi, wide) {
  return $parseIntRadix(s, 10n, lo, hi, wide);
}
export function $parseIntRadix(s, radix, lo, hi, wide) {
  const fail = (m) => ({ ok: false, v: m });
  const base = BigInt(radix);
  if (s.length === 0) return fail("cannot parse integer from empty string");
  let body = s;
  let neg = false;
  if (body[0] === "+" || body[0] === "-") {
    neg = body[0] === "-";
    body = body.slice(1);
    if (neg && lo === 0n) return fail("invalid digit found in string");
    if (body.length === 0) return fail("invalid digit found in string");
  }
  let v = 0n;
  for (const c of body) {
    const d = parseInt(c, 36);
    if (c.length !== 1 || Number.isNaN(d) || BigInt(d) >= base) return fail("invalid digit found in string");
    v = v * base + BigInt(d);
  }
  if (neg) v = -v;
  if (v > hi) return fail("number too large to fit in target type");
  if (v < lo) return fail("number too small to fit in target type");
  return { ok: true, v: wide ? v : Number(v) };
}
export function $parseFloat(s) {
  if (!/^[+-]?(\d+\.?\d*([eE][+-]?\d+)?|\.\d+([eE][+-]?\d+)?|inf|infinity|nan)$/i.test(s)) return { ok: false, v: "invalid float literal" };
  const t = s.toLowerCase().replace(/^([+-]?)(inf|infinity)$/, "$1Infinity").replace(/^[+-]?nan$/, "NaN");
  return { ok: true, v: Number(t) };
}
export const $parseBool = (s) => (s === "true" ? { ok: true, v: true } : s === "false" ? { ok: true, v: false } : { ok: false, v: "provided string was not `true` or `false`" });

// ---------------------------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------------------------
function expandExp(s) {
  const m = /^(-?)(\d+)(?:\.(\d+))?e([+-]\d+)$/.exec(s);
  if (!m) return s;
  const [, sign, ip, fp = "", ex] = m;
  const e = parseInt(ex, 10);
  const digits = ip + fp;
  const point = ip.length + e;
  if (point <= 0) return `${sign}0.${"0".repeat(-point)}${digits}`;
  if (point >= digits.length) return `${sign}${digits}${"0".repeat(point - digits.length)}`;
  return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}

export function $f64(x) {
  if (Number.isNaN(x)) return "NaN";
  if (x === Infinity) return "inf";
  if (x === -Infinity) return "-inf";
  if (Object.is(x, -0)) return "-0";
  return expandExp(String(x));
}

export function $f32(x) {
  if (!Number.isFinite(x) || x === 0) return $f64(x);
  for (let p = 1; p <= 9; p++) {
    const n = Number(x.toPrecision(p));
    if (Math.fround(n) === x) return $f64(n);
  }
  return $f64(x);
}

export function $dbgF64(x, o) {
  if (o) return $fmtFloat(x, o, true, false, false);
  if (!Number.isFinite(x)) return $f64(x);
  const a = Math.abs(x);
  if (a !== 0 && (a >= 1e16 || a < 1e-4)) return x.toExponential().replace("e+", "e");
  const s = $f64(x);
  return s.includes(".") ? s : `${s}.0`;
}
export const $dbgF32 = (x, o) => {
  if (o) return $fmtFloat(x, o, true, true, false);
  if (!Number.isFinite(x) || x === 0) return $dbgF64(x);
  for (let p = 1; p <= 9; p++) {
    const n = Number(x.toPrecision(p));
    if (Math.fround(n) === x) return $dbgF64(n);
  }
  return $dbgF64(x);
};

function escapeChar(c, quote) {
  switch (c) {
    case "\n": return "\\n";
    case "\r": return "\\r";
    case "\t": return "\\t";
    case "\\": return "\\\\";
    case "\0": return "\\0";
    case quote: return `\\${quote}`;
    default: {
      const cp = c.codePointAt(0);
      return cp < 0x20 || cp === 0x7f ? `\\u{${cp.toString(16)}}` : c;
    }
  }
}
export const $dbgStr = (s) => `"${Array.from(s, (c) => escapeChar(c, '"')).join("")}"`;
export const $dbgChar = (c) => `'${escapeChar(c, "'")}'`;
// Debug formatters take `(value, o)`; `o` holds Rust's Formatter options
// (`{ w, fill, align, zero, plus, alt, prec }`, absent = defaults). Like Rust's derived `Debug`, an
// aggregate passes `o` to its elements (`{:5?}` pads every number), and `alt` (`{:#?}`) selects
// the multi-line layout with 4-space indentation.
const indent4 = (s) => s.replace(/\n/g, "\n    ");
export function $dbgList(open, close, items, o) {
  if (!(o && o.alt)) return `${open}${items.join(", ")}${close}`;
  if (items.length === 0) return `${open}${close}`;
  return `${open}\n${items.map((x) => `    ${indent4(x)},\n`).join("")}${close}`;
}
/** `Name(a, b)` (tuple struct / variant); empty name = tuple (`(a,)` for one element). */
export function $dbgTupleS(name, items, o) {
  if (items.length === 0) return name === "" ? "()" : name;
  if (name === "" && items.length === 1 && !(o && o.alt)) return `(${items[0]},)`;
  return name + $dbgList("(", ")", items, o);
}
/** `Name { f: v }` (struct / struct variant). */
export function $dbgStruct(name, fields, o) {
  if (fields.length === 0) return name;
  const items = fields.map(([k, v]) => `${k}: ${v}`);
  if (o && o.alt) return `${name} ${$dbgList("{", "}", items, o)}`;
  return `${name} { ${items.join(", ")} }`;
}
// Map and set iteration is in ascending key order: HashMap order is unspecified (SPEC-EXEC-005),
// and BTreeMap/BTreeSet (which share this representation) require it.
export const $sortedEntries = (m) => Array.from(m).sort((a, b) => $cmp(a[0], b[0]));
export const $sortedKeys = (m) => Array.from(m.keys()).sort($cmp);
export const $sortedValues = (m) => $sortedEntries(m).map((e) => e[1]);
export const $dbgVec = (f) => (v, o) => $dbgList("[", "]", v.map((x) => f(x, o)), o);
export const $dbgSet = (f) => (s, o) => $dbgList("{", "}", $sortedKeys(s).map((x) => f(x, o)), o);
export const $dbgMap = (kf, vf) => (m, o) => $dbgList("{", "}", $sortedEntries(m).map(([k, v]) => `${kf(k, o)}: ${vf(v, o)}`), o);
export const $dbgOpt = (f) => (x, o) => (x === null ? "None" : $dbgTupleS("Some", [f(x.v, o)], o));
export const $dbgRes = (fa, fb) => (r, o) => (r.ok ? $dbgTupleS("Ok", [fa(r.v, o)], o) : $dbgTupleS("Err", [fb(r.v, o)], o));
export const $dbgTuple = (fs) => (t, o) => $dbgTupleS("", t.map((x, i) => fs[i](x, o)), o);
export const $dbgObj = (x, o) => x.$dbg(o);
export const $dbgInt = (x, o) => (o ? $fmtInt(x, o, "", 0) : String(x));
export const $dbgBool = (x, o) => (o ? $fmtStr(String(x), o) : String(x));
export const $dbgUnit = (_x, o) => (o ? $fmtStr("()", o) : "()");
export function $dbgAny(x, o) {
  if (x === null) return "None";
  switch (typeof x) {
    case "string": return $dbgStr(x);
    case "number": return Number.isInteger(x) ? $dbgInt(x, o) : $dbgF64(x, o);
    case "bigint": return $dbgInt(x, o);
    case "boolean": return $dbgBool(x, o);
    default:
  }
  if (Array.isArray(x)) return $dbgList("[", "]", x.map((e) => $dbgAny(e, o)), o);
  if (typeof x.$dbg === "function") return x.$dbg(o);
  return String(x);
}

// Rust `Formatter::pad`: fill/align/width (the `0` and `+` flags do not apply to text).
function padAligned(s, o, defAlign) {
  if (!o || o.w === undefined) return s;
  const len = Array.from(s).length;
  if (len >= o.w) return s;
  const n = o.w - len;
  const fill = o.fill ?? " ";
  const align = o.align ?? defAlign;
  if (align === ">") return fill.repeat(n) + s;
  if (align === "^") return fill.repeat(Math.floor(n / 2)) + s + fill.repeat(n - Math.floor(n / 2));
  return s + fill.repeat(n);
}
/** Display of text with options: precision truncates, then fill/align (default left). */
export const $fmtStr = (s, o) => padAligned(o && o.prec !== undefined ? $trunc(s, o.prec) : s, o, "<");
/** Rust `pad_integral` / `pad_formatted_parts`: sign, prefix, then digits; `0` pads between. */
export function $padNum(neg, prefix, digits, o) {
  const sign = neg ? "-" : o && o.plus ? "+" : "";
  const s = sign + prefix + digits;
  if (!o || o.w === undefined) return s;
  if (o.zero) {
    const n = o.w - Array.from(s).length;
    return n > 0 ? sign + prefix + "0".repeat(n) + digits : s;
  }
  return padAligned(s, o, ">");
}
/** Integer with options; `r` is "" (decimal) or a radix type (`x X b o`, two's complement in `bits`). */
export function $fmtInt(v, o, r, bits) {
  if (!r) {
    const b = BigInt(v);
    return $padNum(b < 0n, "", (b < 0n ? -b : b).toString(), o);
  }
  const prefix = o && o.alt ? { x: "0x", X: "0x", b: "0b", o: "0o" }[r] : "";
  return $padNum(false, prefix, $radix(v, r, bits), o);
}
/** Float with options (`debug`: `{:?}` digits, `exp`: `{:e}`); NaN never has a sign. */
export function $fmtFloat(x, o, debug, f32, exp) {
  if (Number.isNaN(x)) return $padNum(false, "", "NaN", o && { ...o, plus: false });
  const neg = x < 0 || Object.is(x, -0);
  const a = Math.abs(x);
  let digits;
  if (exp) digits = $expFmt(a);
  else if (o && o.prec !== undefined) digits = $fixed(a, o.prec);
  else if (debug) digits = f32 ? $dbgF32(a) : $dbgF64(a);
  else digits = f32 ? $f32(a) : $f64(a);
  return $padNum(neg, "", digits, o);
}

export function $radix(v, r, bits) {
  const u = BigInt.asUintN(bits, BigInt(v));
  const base = { x: 16, X: 16, b: 2, o: 8 }[r];
  const s = u.toString(base);
  return r === "X" ? s.toUpperCase() : s;
}
export const $expFmt = (x) => x.toExponential().replace("e+", "e");
export const $trunc = (s, p) => Array.from(s).slice(0, p).join("");

export function $fixed(x, p) {
  if (!Number.isFinite(x)) return $f64(x);
  const neg = x < 0 || Object.is(x, -0);
  const buf = new DataView(new ArrayBuffer(8));
  buf.setFloat64(0, Math.abs(x));
  const bits = buf.getBigUint64(0);
  const expBits = Number((bits >> 52n) & 0x7ffn);
  let mant = bits & 0xfffffffffffffn;
  let e;
  if (expBits === 0) e = -1074;
  else {
    mant |= 1n << 52n;
    e = expBits - 1075;
  }
  const scale = 10n ** BigInt(p);
  let q;
  if (e >= 0) q = (mant << BigInt(e)) * scale;
  else {
    const num = mant * scale;
    const den = 1n << BigInt(-e);
    q = num / den;
    const r2 = (num % den) * 2n;
    if (r2 > den || (r2 === den && (q & 1n) === 1n)) q += 1n;
  }
  let digits = q.toString().padStart(p + 1, "0");
  const out = p === 0 ? digits : `${digits.slice(0, digits.length - p)}.${digits.slice(digits.length - p)}`;
  return (neg ? "-" : "") + out;
}
