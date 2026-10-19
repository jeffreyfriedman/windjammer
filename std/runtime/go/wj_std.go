// Windjammer std shims for Go: the portable subset of std::{math, strings, fs, time}.
// Signatures mirror std/<module>.wj (see src/codegen/portable/stdlib.rs).

package main

import (
	"fmt"
	"math"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
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


// ---- capabilities (ADR-0021) --------------------------------------------------------------
// The program's grant, installed by generated code only when some std call cannot be proven inside it at compile
// time. Same rules as windjammer-runtime's caps_core (tests/effects/vectors.json pins both). Portable programs reach
// the file system only, so only fs.read / fs.write are matched here.

type wjCapSpec struct {
	Root  string
	Allow map[string][]string
	Deny  []string
}

var wjCapGrant *wjCapSpec

func wjstd_caps_install(spec wjCapSpec) {
	if abs, err := filepath.Abs(spec.Root); err == nil {
		spec.Root = abs
	}
	wjCapGrant = &spec
}

func wjCapFold(s string) string {
	if runtime.GOOS == "darwin" || runtime.GOOS == "windows" {
		return strings.ToLower(s)
	}
	return s
}

func wjCapComps(p string) []string {
	var out []string
	for _, c := range strings.Split(filepath.ToSlash(p), "/") {
		if c != "" {
			out = append(out, wjCapFold(c))
		}
	}
	return out
}

func wjCapCanonical(p, base string) string {
	lex := p
	if !filepath.IsAbs(p) {
		lex = filepath.Join(base, p)
	}
	lex = filepath.Clean(lex)
	existing := lex
	var rest []string
	for {
		if c, err := filepath.EvalSymlinks(existing); err == nil {
			for i := len(rest) - 1; i >= 0; i-- {
				c = filepath.Join(c, rest[i])
			}
			return c
		}
		parent := filepath.Dir(existing)
		if parent == existing {
			return lex
		}
		rest = append(rest, filepath.Base(existing))
		existing = parent
	}
}

func wjCapGlob(pat, s string) bool {
	parts := strings.Split(pat, "*")
	for i, x := range parts {
		parts[i] = regexp.QuoteMeta(x)
	}
	return regexp.MustCompile("^" + strings.Join(parts, ".*") + "$").MatchString(s)
}

func wjCapTail(tail, rest []string) bool {
	if len(tail) == 0 {
		return len(rest) == 0
	}
	if tail[0] == "**" {
		for j := 0; j <= len(rest); j++ {
			if wjCapTail(tail[1:], rest[j:]) {
				return true
			}
		}
		return false
	}
	return len(rest) > 0 && wjCapGlob(wjCapFold(tail[0]), rest[0]) && wjCapTail(tail[1:], rest[1:])
}

func wjCapPathMatches(pat, target, root string) bool {
	var segs []string
	for _, s := range strings.Split(pat, "/") {
		if s != "" && s != "." {
			segs = append(segs, s)
		}
	}
	first := len(segs)
	for i, s := range segs {
		if strings.Contains(s, "*") {
			first = i
			break
		}
	}
	prefix := strings.Join(segs[:first], "/")
	if strings.HasPrefix(pat, "/") {
		prefix = "/" + prefix
	}
	if prefix == "" {
		prefix = "."
	}
	pc := wjCapComps(wjCapCanonical(prefix, root))
	t := wjCapComps(target)
	if len(t) < len(pc) {
		return false
	}
	for i, c := range pc {
		if t[i] != c {
			return false
		}
	}
	tail, rest := segs[first:], t[len(pc):]
	if len(tail) == 0 {
		return len(rest) == 0
	}
	return wjCapTail(tail, rest)
}

func wjCapProtected(p string) bool {
	name := filepath.Base(p)
	return name == "wj.toml" || strings.HasSuffix(name, ".wj-capabilities.lock") || strings.HasSuffix(name, ".wj")
}

func wjCapDisplay(root, res string) string {
	cwd, _ := os.Getwd()
	abs := res
	if !filepath.IsAbs(res) {
		abs = filepath.Join(cwd, res)
	}
	abs = filepath.Clean(abs)
	rel, err := filepath.Rel(root, abs)
	if err != nil || strings.HasPrefix(rel, "..") {
		return abs
	}
	if rel == "." {
		return "."
	}
	return "./" + filepath.ToSlash(rel)
}

// wjCapDecide returns "" when the use is allowed, otherwise the denial text.
func wjCapDecide(g wjCapSpec, kind string, resource *string) string {
	deny := func(scopeText string) string {
		subject, entry := kind, kind+" = true"
		if resource != nil {
			shown := wjCapDisplay(g.Root, *resource)
			subject = kind + "(" + shown + ")"
			entry = kind + ` = ["` + shown + `"]`
		}
		return fmt.Sprintf("error: capability denied: %s\n  = grant in force: %s\n  = to allow: add `%s` under [capabilities] in wj.toml\n", subject, scopeText, entry)
	}
	for _, d := range g.Deny {
		if d == kind {
			return deny(kind + " is denied")
		}
	}
	scope, ok := g.Allow[kind]
	if !ok {
		if kind == "time" || kind == "random" || kind == "console" {
			return ""
		}
		return deny(kind + " not granted")
	}
	if len(scope) == 1 && scope[0] == "*" {
		return ""
	}
	scopeText := kind + " = [" + strings.Join(scope, ", ") + "]"
	if resource == nil {
		return deny(scopeText)
	}
	cwd, _ := os.Getwd()
	target := wjCapCanonical(*resource, cwd)
	for _, pat := range scope {
		if kind == "fs.write" && wjCapProtected(target) && strings.Contains(pat, "*") {
			continue
		}
		if wjCapPathMatches(pat, target, g.Root) {
			return ""
		}
	}
	return deny(scopeText)
}

// wjstd_caps_vector is the conformance-vector entry point: "allow" or "deny" for one use under spec.
func wjstd_caps_vector(spec wjCapSpec, kind string, resource *string) string {
	if abs, err := filepath.Abs(spec.Root); err == nil {
		spec.Root = abs
	}
	if wjCapDecide(spec, kind, resource) == "" {
		return "allow"
	}
	return "deny"
}

func wjCapsFs(kind, p string) {
	if wjCapGrant == nil {
		return
	}
	if d := wjCapDecide(*wjCapGrant, kind, &p); d != "" {
		fmt.Fprint(os.Stderr, d)
		os.Exit(126)
	}
}

// ---- fs -----------------------------------------------------------------------------------

func wjstd_fs_result(err error) Result[struct{}, string] {
	if err != nil {
		return Result[struct{}, string]{E: err.Error()}
	}
	return Result[struct{}, string]{Ok: true}
}

func wjstd_fs_read_to_string(p string) Result[string, string] {
	wjCapsFs("fs.read", p)
	b, err := os.ReadFile(p)
	if err != nil {
		return Result[string, string]{E: err.Error()}
	}
	return Result[string, string]{Ok: true, V: string(b)}
}

func wjstd_fs_write(p, c string) Result[struct{}, string] {
	wjCapsFs("fs.write", p)
	return wjstd_fs_result(os.WriteFile(p, []byte(c), 0o644))
}

func wjstd_fs_append(p, c string) Result[struct{}, string] {
	wjCapsFs("fs.write", p)
	f, err := os.OpenFile(p, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return wjstd_fs_result(err)
	}
	defer f.Close()
	_, err = f.WriteString(c)
	return wjstd_fs_result(err)
}

func wjstd_fs_remove_file(p string) Result[struct{}, string] {
	wjCapsFs("fs.write", p)
	return wjstd_fs_result(os.Remove(p))
}
func wjstd_fs_create_dir_all(p string) Result[struct{}, string] {
	wjCapsFs("fs.write", p)
	return wjstd_fs_result(os.MkdirAll(p, 0o755))
}
func wjstd_fs_remove_dir_all(p string) Result[struct{}, string] {
	wjCapsFs("fs.write", p)
	return wjstd_fs_result(os.RemoveAll(p))
}
func wjstd_fs_exists(p string) bool {
	wjCapsFs("fs.read", p)
	_, err := os.Stat(p)
	return err == nil
}
func wjstd_fs_is_file(p string) bool {
	wjCapsFs("fs.read", p)
	st, err := os.Stat(p)
	return err == nil && st.Mode().IsRegular()
}
func wjstd_fs_is_dir(p string) bool {
	wjCapsFs("fs.read", p)
	st, err := os.Stat(p)
	return err == nil && st.IsDir()
}

// ---- time ---------------------------------------------------------------------------------

func wjstd_time_now_millis() int64      { return time.Now().UnixMilli() }
func wjstd_time_timestamp() int64       { return time.Now().Unix() }
func wjstd_time_now_unix() int64        { return time.Now().Unix() }
func wjstd_time_sleep_millis(ms uint64) { time.Sleep(time.Duration(ms) * time.Millisecond) }
