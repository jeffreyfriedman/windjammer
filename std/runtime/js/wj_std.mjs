// Windjammer std shims for JavaScript: the portable subset of std::{math, strings, fs, time}.
// Signatures mirror std/<module>.wj (see src/codegen/portable/stdlib.rs). usize/i64/u64 are BigInt.

import { $panic, $strSlice, $strlen, $split, $lines } from "./wj_core.mjs";

const nodeFs = typeof process !== "undefined" && process.versions && process.versions.node ? await import("node:fs") : null;

// ---- math ---------------------------------------------------------------------------------
export const $std_math_sqrt = Math.sqrt;
export const $std_math_cbrt = Math.cbrt;
export const $std_math_round = (x) => (x < 0 ? -Math.round(-x) : Math.round(x));
export const $std_math_floor = Math.floor;
export const $std_math_ceil = Math.ceil;
export const $std_math_trunc = Math.trunc;
export const $std_math_sin = Math.sin;
export const $std_math_cos = Math.cos;
export const $std_math_tan = Math.tan;
export const $std_math_asin = Math.asin;
export const $std_math_acos = Math.acos;
export const $std_math_atan = Math.atan;
export const $std_math_exp = Math.exp;
export const $std_math_ln = Math.log;
export const $std_math_log2 = Math.log2;
export const $std_math_log10 = Math.log10;
export const $std_math_abs_f64 = Math.abs;
export const $std_math_signum = (x) => (Number.isNaN(x) ? NaN : x < 0 || Object.is(x, -0) ? -1 : 1);
export const $std_math_to_radians = (x) => (x * Math.PI) / 180;
export const $std_math_to_degrees = (x) => (x * 180) / Math.PI;
export const $std_math_pow = Math.pow;
export const $std_math_atan2 = Math.atan2;
export const $std_math_hypot = Math.hypot;
export const $std_math_min_f64 = (a, b) => (Number.isNaN(a) ? b : Number.isNaN(b) ? a : Math.min(a, b));
export const $std_math_max_f64 = (a, b) => (Number.isNaN(a) ? b : Number.isNaN(b) ? a : Math.max(a, b));
export const $std_math_clamp_f64 = (x, lo, hi) => $std_math_min_f64($std_math_max_f64(x, lo), hi);
export const $std_math_abs_i64 = (x) => (x < 0n ? -x : x);
export const $std_math_min_i64 = (a, b) => (a < b ? a : b);
export const $std_math_max_i64 = (a, b) => (a > b ? a : b);
export const $std_math_clamp_i64 = (x, lo, hi) => (x < lo ? lo : x > hi ? hi : x);
export const $std_math_is_nan = Number.isNaN;
export const $std_math_is_infinite = (x) => Math.abs(x) === Infinity;
export const $std_math_is_finite = Number.isFinite;

// ---- strings ------------------------------------------------------------------------------
export const $std_strings_to_upper = (s) => s.toUpperCase();
export const $std_strings_to_lower = (s) => s.toLowerCase();
export const $std_strings_trim = (s) => s.trim();
export const $std_strings_trim_start = (s) => s.trimStart();
export const $std_strings_trim_end = (s) => s.trimEnd();
export const $std_strings_split = (s, d) => $split(s, d);
export const $std_strings_split_lines = (s) => $lines(s);
export const $std_strings_split_whitespace = (s) => s.split(/\s+/).filter((x) => x.length > 0);
export const $std_strings_join = (parts, d) => parts.join(d);
export const $std_strings_chars = (s) => Array.from(s);
export const $std_strings_from_chars = (cs) => cs.join("");
export const $std_strings_substring_chars = (s, a, b) => Array.from(s).slice(Number(a), Number(b)).join("");
export const $std_strings_substring = (s, a, b) => $strSlice(s, a, b, false);
export const $std_strings_is_empty = (s) => s.length === 0;
export const $std_strings_starts_with = (s, p) => s.startsWith(p);
export const $std_strings_ends_with = (s, p) => s.endsWith(p);
export const $std_strings_contains = (s, p) => s.includes(p);
export const $std_strings_replace = (s, a, b) => s.split(a).join(b);
export const $std_strings_len = (s) => BigInt($strlen(s));
export const $std_strings_char_count = (s) => BigInt(Array.from(s).length);
export const $std_strings_repeat = (s, n) => s.repeat(Number(n));
export const $std_strings_byte_at = (s, i) => {
  const b = new TextEncoder().encode(s);
  const n = Number(i);
  return n < b.length ? b[n] : $panic("index out of bounds");
};

// ---- fs -----------------------------------------------------------------------------------
const needFs = () => (nodeFs ? nodeFs : $panic("std::fs is not available in this JavaScript environment"));
const attempt = (f) => {
  try {
    return { ok: true, v: f() };
  } catch (e) {
    return { ok: false, v: String((e && e.message) || e) };
  }
};
export const $std_fs_read_to_string = (p) => attempt(() => needFs().readFileSync(p, "utf8"));
export const $std_fs_write = (p, c) => attempt(() => void needFs().writeFileSync(p, c));
export const $std_fs_append = (p, c) => attempt(() => void needFs().appendFileSync(p, c));
export const $std_fs_remove_file = (p) => attempt(() => void needFs().unlinkSync(p));
export const $std_fs_create_dir_all = (p) => attempt(() => void needFs().mkdirSync(p, { recursive: true }));
export const $std_fs_remove_dir_all = (p) => attempt(() => void needFs().rmSync(p, { recursive: true, force: true }));
export const $std_fs_exists = (p) => needFs().existsSync(p);
export const $std_fs_is_file = (p) => { try { return needFs().statSync(p).isFile(); } catch { return false; } };
export const $std_fs_is_dir = (p) => { try { return needFs().statSync(p).isDirectory(); } catch { return false; } };

// ---- time ---------------------------------------------------------------------------------
export const $std_time_now_millis = () => BigInt(Date.now());
export const $std_time_timestamp = () => BigInt(Math.floor(Date.now() / 1000));
export const $std_time_now_unix = () => BigInt(Math.floor(Date.now() / 1000));
export const $std_time_sleep_millis = (ms) => {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, Number(ms));
};
