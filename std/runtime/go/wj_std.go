// Windjammer std shims for Go: the portable subset of std::{math, strings, fs, time}.
// Signatures mirror std/<module>.wj (see src/codegen/portable/stdlib.rs).

package main

import (
	"math"
	"os"
	"strings"
	"time"
	"unicode/utf8"
)

// ---- math ---------------------------------------------------------------------------------

func wjstd_math_sqrt(x float64) float64       { return math.Sqrt(x) }
func wjstd_math_cbrt(x float64) float64       { return math.Cbrt(x) }
func wjstd_math_round(x float64) float64      { return math.Round(x) }
func wjstd_math_floor(x float64) float64      { return math.Floor(x) }
func wjstd_math_ceil(x float64) float64       { return math.Ceil(x) }
func wjstd_math_trunc(x float64) float64      { return math.Trunc(x) }
func wjstd_math_sin(x float64) float64        { return math.Sin(x) }
func wjstd_math_cos(x float64) float64        { return math.Cos(x) }
func wjstd_math_tan(x float64) float64        { return math.Tan(x) }
func wjstd_math_asin(x float64) float64       { return math.Asin(x) }
func wjstd_math_acos(x float64) float64       { return math.Acos(x) }
func wjstd_math_atan(x float64) float64       { return math.Atan(x) }
func wjstd_math_exp(x float64) float64        { return math.Exp(x) }
func wjstd_math_ln(x float64) float64         { return math.Log(x) }
func wjstd_math_log2(x float64) float64       { return math.Log2(x) }
func wjstd_math_log10(x float64) float64      { return math.Log10(x) }
func wjstd_math_abs_f64(x float64) float64    { return math.Abs(x) }
func wjstd_math_signum(x float64) float64     { return wjFsignum(x) }
func wjstd_math_to_radians(x float64) float64 { return x * math.Pi / 180 }
func wjstd_math_to_degrees(x float64) float64 { return x * 180 / math.Pi }
func wjstd_math_pow(x, y float64) float64     { return math.Pow(x, y) }
func wjstd_math_atan2(y, x float64) float64   { return math.Atan2(y, x) }
func wjstd_math_hypot(x, y float64) float64   { return math.Hypot(x, y) }
func wjstd_math_min_f64(a, b float64) float64 { return wjFmin(a, b) }
func wjstd_math_max_f64(a, b float64) float64 { return wjFmax(a, b) }
func wjstd_math_clamp_f64(x, lo, hi float64) float64 {
	return wjFmin(wjFmax(x, lo), hi)
}
func wjstd_math_abs_i64(x int64) int64 { return wjAbs(x) }
func wjstd_math_min_i64(a, b int64) int64 {
	if a < b {
		return a
	}
	return b
}
func wjstd_math_max_i64(a, b int64) int64 {
	if a > b {
		return a
	}
	return b
}
func wjstd_math_clamp_i64(x, lo, hi int64) int64 {
	return wjstd_math_min_i64(wjstd_math_max_i64(x, lo), hi)
}
func wjstd_math_is_nan(x float64) bool      { return x != x }
func wjstd_math_is_infinite(x float64) bool { return math.IsInf(x, 0) }
func wjstd_math_is_finite(x float64) bool   { return !math.IsInf(x, 0) && x == x }

// ---- strings ------------------------------------------------------------------------------

func wjstd_strings_to_upper(s string) string     { return strings.ToUpper(s) }
func wjstd_strings_to_lower(s string) string     { return strings.ToLower(s) }
func wjstd_strings_trim(s string) string         { return wjTrim(s) }
func wjstd_strings_trim_start(s string) string   { return wjTrimStart(s) }
func wjstd_strings_trim_end(s string) string     { return wjTrimEnd(s) }
func wjstd_strings_split(s, d string) []string   { return wjStrSplit(s, d) }
func wjstd_strings_split_lines(s string) []string { return wjLines(s) }
func wjstd_strings_split_whitespace(s string) []string {
	return strings.Fields(s)
}
func wjstd_strings_join(p []string, d string) string { return strings.Join(p, d) }
func wjstd_strings_chars(s string) []Char            { return wjChars(s) }
func wjstd_strings_from_chars(cs []Char) string {
	var b strings.Builder
	for _, c := range cs {
		b.WriteRune(rune(c))
	}
	return b.String()
}
func wjstd_strings_substring_chars(s string, a, b uint) string {
	rs := []rune(s)
	if a > b || int(b) > len(rs) {
		wjPanic("character range out of bounds")
	}
	return string(rs[a:b])
}
func wjstd_strings_substring(s string, a, b uint) string { return wjStrSlice(s, a, b, false) }
func wjstd_strings_is_empty(s string) bool                { return len(s) == 0 }
func wjstd_strings_starts_with(s, p string) bool          { return strings.HasPrefix(s, p) }
func wjstd_strings_ends_with(s, p string) bool            { return strings.HasSuffix(s, p) }
func wjstd_strings_contains(s, p string) bool             { return strings.Contains(s, p) }
func wjstd_strings_replace(s, a, b string) string         { return strings.ReplaceAll(s, a, b) }
func wjstd_strings_len(s string) uint                     { return uint(len(s)) }
func wjstd_strings_char_count(s string) uint              { return uint(utf8.RuneCountInString(s)) }
func wjstd_strings_repeat(s string, n uint) string        { return strings.Repeat(s, int(n)) }
func wjstd_strings_byte_at(s string, i uint) uint8 {
	if int(i) >= len(s) {
		wjPanic("index out of bounds")
	}
	return s[i]
}

// ---- fs -----------------------------------------------------------------------------------

func wjstd_fs_result(err error) Result[struct{}, string] {
	if err != nil {
		return Result[struct{}, string]{E: err.Error()}
	}
	return Result[struct{}, string]{Ok: true}
}

func wjstd_fs_read_to_string(p string) Result[string, string] {
	b, err := os.ReadFile(p)
	if err != nil {
		return Result[string, string]{E: err.Error()}
	}
	return Result[string, string]{Ok: true, V: string(b)}
}

func wjstd_fs_write(p, c string) Result[struct{}, string] { return wjstd_fs_result(os.WriteFile(p, []byte(c), 0o644)) }

func wjstd_fs_append(p, c string) Result[struct{}, string] {
	f, err := os.OpenFile(p, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return wjstd_fs_result(err)
	}
	defer f.Close()
	_, err = f.WriteString(c)
	return wjstd_fs_result(err)
}

func wjstd_fs_remove_file(p string) Result[struct{}, string]     { return wjstd_fs_result(os.Remove(p)) }
func wjstd_fs_create_dir_all(p string) Result[struct{}, string]  { return wjstd_fs_result(os.MkdirAll(p, 0o755)) }
func wjstd_fs_remove_dir_all(p string) Result[struct{}, string]  { return wjstd_fs_result(os.RemoveAll(p)) }
func wjstd_fs_exists(p string) bool {
	_, err := os.Stat(p)
	return err == nil
}
func wjstd_fs_is_file(p string) bool {
	st, err := os.Stat(p)
	return err == nil && st.Mode().IsRegular()
}
func wjstd_fs_is_dir(p string) bool {
	st, err := os.Stat(p)
	return err == nil && st.IsDir()
}

// ---- time ---------------------------------------------------------------------------------

func wjstd_time_now_millis() int64      { return time.Now().UnixMilli() }
func wjstd_time_timestamp() int64       { return time.Now().Unix() }
func wjstd_time_now_unix() int64        { return time.Now().Unix() }
func wjstd_time_sleep_millis(ms uint64) { time.Sleep(time.Duration(ms) * time.Millisecond) }
