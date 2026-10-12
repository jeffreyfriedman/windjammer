// Windjammer Go runtime core. Package main, Go >= 1.22 (generics).
//
// Representation contract (see src/codegen/go/MODULE.md):
//   integers -> exact Go widths (usize = uint, isize = int); arithmetic panics on overflow
//   char -> Char (rune); Vec -> slice; HashMap -> map; HashSet -> map[T]struct{}
//   Option -> Option[T]; Result -> Result[T, E]; tuples -> TupleN[...]
//   structs -> Go structs with exported fields; enums -> interface + one struct per variant
// Panics (explicit, overflow, index, division by zero) exit with status 101 like Rust.

package main

import (
	"bufio"
	"cmp"
	"fmt"
	"math"
	"math/bits"
	"os"
	"reflect"
	"slices"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
	"unsafe"
)

// ---------------------------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------------------------

type Char rune

type integer interface {
	~int | ~int8 | ~int16 | ~int32 | ~int64 | ~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64
}

type number interface {
	integer | ~float32 | ~float64
}

type Option[T any] struct {
	Ok bool
	V  T
}

type Result[T any, E any] struct {
	Ok bool
	V  T
	E  E
}

type Tuple2[A, B any] struct {
	F0 A
	F1 B
}
type Tuple3[A, B, C any] struct {
	F0 A
	F1 B
	F2 C
}
type Tuple4[A, B, C, D any] struct {
	F0 A
	F1 B
	F2 C
	F3 D
}
type Tuple5[A, B, C, D, E any] struct {
	F0 A
	F1 B
	F2 C
	F3 D
	F4 E
}
type Tuple6[A, B, C, D, E, F any] struct {
	F0 A
	F1 B
	F2 C
	F3 D
	F4 E
	F5 F
}

func (o Option[T]) wjDbgF(f *wjFmt) string {
	if !o.Ok {
		return "None"
	}
	return wjDbgTupleS(f, "Some", o.V)
}

func (r Result[T, E]) wjDbgF(f *wjFmt) string {
	if r.Ok {
		return wjDbgTupleS(f, "Ok", r.V)
	}
	return wjDbgTupleS(f, "Err", r.E)
}

func (t Tuple2[A, B]) wjDbgF(f *wjFmt) string       { return wjDbgTupleS(f, "", t.F0, t.F1) }
func (t Tuple3[A, B, C]) wjDbgF(f *wjFmt) string    { return wjDbgTupleS(f, "", t.F0, t.F1, t.F2) }
func (t Tuple4[A, B, C, D]) wjDbgF(f *wjFmt) string { return wjDbgTupleS(f, "", t.F0, t.F1, t.F2, t.F3) }
func (t Tuple5[A, B, C, D, E]) wjDbgF(f *wjFmt) string {
	return wjDbgTupleS(f, "", t.F0, t.F1, t.F2, t.F3, t.F4)
}
func (t Tuple6[A, B, C, D, E, F]) wjDbgF(f *wjFmt) string {
	return wjDbgTupleS(f, "", t.F0, t.F1, t.F2, t.F3, t.F4, t.F5)
}

// ---------------------------------------------------------------------------------------------
// Process: output, panics, entry point
// ---------------------------------------------------------------------------------------------

var wjOut = bufio.NewWriterSize(os.Stdout, 1<<16)

type wjPanicMsg string

func wjPanic(msg string) {
	panic(wjPanicMsg(msg))
}

func wjPrint(s string) { wjOut.WriteString(s) }

func wjEprint(s string) {
	wjOut.Flush()
	os.Stderr.WriteString(s)
}

func wjRun(f func()) {
	code := 0
	func() {
		defer func() {
			if r := recover(); r != nil {
				wjOut.Flush()
				fmt.Fprintf(os.Stderr, "thread 'main' panicked:\n%v\n", r)
				code = 101
			}
		}()
		f()
	}()
	wjOut.Flush()
	os.Exit(code)
}

func wjOverflow(op string) { wjPanic("attempt to " + op + " with overflow") }

// ---------------------------------------------------------------------------------------------
// Integers
// ---------------------------------------------------------------------------------------------

func isSigned[T integer]() bool {
	var z T
	return z-1 < z
}

func bitsOf[T integer]() int {
	var z T
	return int(unsafe.Sizeof(z)) * 8
}

func minVal[T integer]() T {
	if !isSigned[T]() {
		return 0
	}
	return T(1) << (bitsOf[T]() - 1)
}

func maxVal[T integer]() T {
	if !isSigned[T]() {
		return ^T(0)
	}
	return ^minVal[T]()
}

func wjWAdd[T integer](a, b T) T { return a + b }
func wjWSub[T integer](a, b T) T { return a - b }
func wjWMul[T integer](a, b T) T { return a * b }
func wjWNeg[T integer](a T) T     { return -a }

func addC[T integer](a, b T) (T, bool) {
	c := a + b
	if isSigned[T]() {
		return c, (a^c)&(b^c) < 0
	}
	return c, c < a
}

func subC[T integer](a, b T) (T, bool) {
	c := a - b
	if isSigned[T]() {
		return c, (a^b)&(a^c) < 0
	}
	return c, b > a
}

func mulC[T integer](a, b T) (T, bool) {
	if a == 0 || b == 0 {
		return 0, false
	}
	c := a * b
	if isSigned[T]() {
		negOne := ^T(0)
		if (a == negOne && b == minVal[T]()) || (b == negOne && a == minVal[T]()) {
			return c, true
		}
	}
	return c, c/b != a
}

func wjAdd[T integer](a, b T) T {
	c, o := addC(a, b)
	if o {
		wjOverflow("add")
	}
	return c
}

func wjSub[T integer](a, b T) T {
	c, o := subC(a, b)
	if o {
		wjOverflow("subtract")
	}
	return c
}

func wjMul[T integer](a, b T) T {
	c, o := mulC(a, b)
	if o {
		wjOverflow("multiply")
	}
	return c
}

func wjDiv[T integer](a, b T) T {
	if b == 0 {
		wjPanic("attempt to divide by zero")
	}
	if isSigned[T]() && b == ^T(0) && a == minVal[T]() {
		wjOverflow("divide")
	}
	return a / b
}

func wjRem[T integer](a, b T) T {
	if b == 0 {
		wjPanic("attempt to calculate the remainder with a divisor of zero")
	}
	if isSigned[T]() && b == ^T(0) {
		if a == minVal[T]() {
			wjOverflow("calculate the remainder")
		}
		return 0
	}
	return a % b
}

func wjNeg[T integer](a T) T {
	if isSigned[T]() && a == minVal[T]() {
		wjOverflow("negate")
	}
	return -a
}

func wjShl[T integer, S integer](a T, n S) T {
	if n < 0 || uint64(n) >= uint64(bitsOf[T]()) {
		wjPanic("attempt to shift left with overflow")
	}
	return a << uint64(n)
}

func wjShr[T integer, S integer](a T, n S) T {
	if n < 0 || uint64(n) >= uint64(bitsOf[T]()) {
		wjPanic("attempt to shift right with overflow")
	}
	return a >> uint64(n)
}

func wjF2I[T integer](f float64) T {
	if f != f {
		return 0
	}
	if f >= float64(maxVal[T]()) {
		return maxVal[T]()
	}
	if f <= float64(minVal[T]()) {
		return minVal[T]()
	}
	return T(f)
}

func wjB2I[T integer](b bool) T {
	if b {
		return 1
	}
	return 0
}

func wjAbs[T integer](a T) T {
	if isSigned[T]() && a < 0 {
		return wjNeg(a)
	}
	return a
}

func wjSignum[T integer](a T) T {
	if a > 0 {
		return 1
	}
	if isSigned[T]() && a < 0 {
		return ^T(0)
	}
	return 0
}

func wjPow[T integer, N integer](a T, n N) T {
	var r T = 1
	for i := N(0); i < n; i++ {
		r = wjMul(r, a)
	}
	return r
}

func wjMin2[T cmp.Ordered](a, b T) T {
	if a < b {
		return a
	}
	return b
}

func wjMax2[T cmp.Ordered](a, b T) T {
	if a > b {
		return a
	}
	return b
}

func wjSatAdd[T integer](a, b T) T {
	c, o := addC(a, b)
	if !o {
		return c
	}
	if isSigned[T]() && b < 0 {
		return minVal[T]()
	}
	return maxVal[T]()
}

func wjSatSub[T integer](a, b T) T {
	c, o := subC(a, b)
	if !o {
		return c
	}
	if isSigned[T]() && b < 0 {
		return maxVal[T]()
	}
	return minVal[T]()
}

func wjSatMul[T integer](a, b T) T {
	c, o := mulC(a, b)
	if !o {
		return c
	}
	if isSigned[T]() && (a < 0) != (b < 0) {
		return minVal[T]()
	}
	return maxVal[T]()
}

func wjCheckedAdd[T integer](a, b T) Option[T] {
	c, o := addC(a, b)
	return Option[T]{Ok: !o, V: c}
}

func wjCheckedSub[T integer](a, b T) Option[T] {
	c, o := subC(a, b)
	return Option[T]{Ok: !o, V: c}
}

func wjCheckedMul[T integer](a, b T) Option[T] {
	c, o := mulC(a, b)
	return Option[T]{Ok: !o, V: c}
}

func wjCheckedDiv[T integer](a, b T) Option[T] {
	if b == 0 || (isSigned[T]() && b == ^T(0) && a == minVal[T]()) {
		return Option[T]{}
	}
	return Option[T]{Ok: true, V: a / b}
}

func wjCheckedRem[T integer](a, b T) Option[T] {
	if b == 0 || (isSigned[T]() && b == ^T(0) && a == minVal[T]()) {
		return Option[T]{}
	}
	return Option[T]{Ok: true, V: a % b}
}

func wjRemEuclid[T integer](a, b T) T {
	r := wjRem(a, b)
	if isSigned[T]() && r < 0 {
		if b < 0 {
			return r - b
		}
		return r + b
	}
	return r
}

func wjDivEuclid[T integer](a, b T) T {
	q := wjDiv(a, b)
	if isSigned[T]() && wjRem(a, b) < 0 {
		if b > 0 {
			return q - 1
		}
		return q + 1
	}
	return q
}

func wjAbsDiff[T integer](a, b T) T {
	if a > b {
		return a - b
	}
	return b - a
}

func widthMask[T integer]() uint64 {
	return ^uint64(0) >> (64 - bitsOf[T]())
}

func wjPopcnt[T integer](a T) uint32 { return uint32(bits.OnesCount64(uint64(a) & widthMask[T]())) }

func wjClz[T integer](a T) uint32 {
	return uint32(bits.LeadingZeros64(uint64(a)&widthMask[T]())) - uint32(64-bitsOf[T]())
}

func wjCtz[T integer](a T) uint32 {
	x := uint64(a) & widthMask[T]()
	if x == 0 {
		return uint32(bitsOf[T]())
	}
	return uint32(bits.TrailingZeros64(x))
}

func wjRotl[T integer, N integer](a T, n N) T {
	w := uint64(bitsOf[T]())
	k := uint64(n) % w
	x := uint64(a) & widthMask[T]()
	return T(((x << k) | (x >> ((w - k) % w))) & widthMask[T]())
}

func wjRotr[T integer, N integer](a T, n N) T {
	w := uint64(bitsOf[T]())
	k := uint64(n) % w
	x := uint64(a) & widthMask[T]()
	return T(((x >> k) | (x << ((w - k) % w))) & widthMask[T]())
}

// ---------------------------------------------------------------------------------------------
// Floats
// ---------------------------------------------------------------------------------------------

const wjPi = math.Pi

func wjNaN() float64          { return math.NaN() }
func wjNegZero() float64      { return math.Copysign(0, -1) }
func wjFneg[T ~float32 | ~float64](x T) T { return -x }
func wjInf(sign int) float64  { return math.Inf(sign) }
func wjFadd[T ~float32 | ~float64](a, b T) T { return a + b }
func wjFsub[T ~float32 | ~float64](a, b T) T { return a - b }
func wjFmul[T ~float32 | ~float64](a, b T) T { return a * b }
func wjFdiv[T ~float32 | ~float64](a, b T) T { return a / b }
func wjFmod[T ~float32 | ~float64](a, b T) T { return T(math.Mod(float64(a), float64(b))) }
func wjMath_sqrt(x float64) float64  { return math.Sqrt(x) }
func wjMath_abs(x float64) float64   { return math.Abs(x) }
func wjMath_floor(x float64) float64 { return math.Floor(x) }
func wjMath_ceil(x float64) float64  { return math.Ceil(x) }
func wjMath_trunc(x float64) float64 { return math.Trunc(x) }
func wjMath_sin(x float64) float64   { return math.Sin(x) }
func wjMath_cos(x float64) float64   { return math.Cos(x) }
func wjMath_tan(x float64) float64   { return math.Tan(x) }
func wjMath_asin(x float64) float64  { return math.Asin(x) }
func wjMath_acos(x float64) float64  { return math.Acos(x) }
func wjMath_atan(x float64) float64  { return math.Atan(x) }
func wjMath_exp(x float64) float64   { return math.Exp(x) }
func wjMath_cbrt(x float64) float64  { return math.Cbrt(x) }
func wjMath_log(x float64) float64   { return math.Log(x) }
func wjMath_log10(x float64) float64 { return math.Log10(x) }
func wjMath_log2(x float64) float64  { return math.Log2(x) }
func wjMath_pow(x, y float64) float64 { return math.Pow(x, y) }
func wjMath_atan2(y, x float64) float64 { return math.Atan2(y, x) }
func wjMath_hypot(x, y float64) float64 { return math.Hypot(x, y) }
func wjRound(x float64) float64        { return math.Round(x) }
func wjFmin(a, b float64) float64 {
	if a != a {
		return b
	}
	if b != b {
		return a
	}
	return math.Min(a, b)
}
func wjFmax(a, b float64) float64 {
	if a != a {
		return b
	}
	if b != b {
		return a
	}
	return math.Max(a, b)
}
func wjFsignum(x float64) float64 {
	if x != x {
		return x
	}
	if math.Signbit(x) {
		return -1
	}
	return 1
}
func wjIsNaN(x float64) bool    { return x != x }
func wjIsInf(x float64) bool    { return math.IsInf(x, 0) }
func wjIsFinite(x float64) bool { return !math.IsInf(x, 0) && x == x }
func wjSignNeg(x float64) bool  { return math.Signbit(x) }

// ---------------------------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------------------------

func wjItoa[T integer](x T) string {
	if isSigned[T]() {
		return strconv.FormatInt(int64(x), 10)
	}
	return strconv.FormatUint(uint64(x), 10)
}

func wjBool(b bool) string { return strconv.FormatBool(b) }

func wjF64(x float64) string {
	switch {
	case x != x:
		return "NaN"
	case math.IsInf(x, 1):
		return "inf"
	case math.IsInf(x, -1):
		return "-inf"
	}
	return strconv.FormatFloat(x, 'f', -1, 64)
}

func wjF32(x float32) string {
	f := float64(x)
	switch {
	case f != f:
		return "NaN"
	case math.IsInf(f, 1):
		return "inf"
	case math.IsInf(f, -1):
		return "-inf"
	}
	return strconv.FormatFloat(f, 'f', -1, 32)
}

func wjFixed(x float64, p int) string {
	if x != x || math.IsInf(x, 0) {
		return wjF64(x)
	}
	return strconv.FormatFloat(x, 'f', p, 64)
}

func wjDbgFloat(x float64, bits int) string {
	if x != x || math.IsInf(x, 0) {
		return wjF64(x)
	}
	a := math.Abs(x)
	if a != 0 && (a >= 1e16 || a < 1e-4) {
		s := strconv.FormatFloat(x, 'e', -1, bits)
		mant, exp, _ := strings.Cut(s, "e")
		e, _ := strconv.Atoi(exp)
		return mant + "e" + strconv.Itoa(e)
	}
	s := strconv.FormatFloat(x, 'f', -1, bits)
	if !strings.Contains(s, ".") {
		s += ".0"
	}
	return s
}

func wjExpFmt(x float64) string {
	s := strconv.FormatFloat(x, 'e', -1, 64)
	mant, exp, _ := strings.Cut(s, "e")
	e, _ := strconv.Atoi(exp)
	return mant + "e" + strconv.Itoa(e)
}

func wjRadix[T integer](x T, r rune) string {
	u := uint64(x) & widthMask[T]()
	base := map[rune]int{'x': 16, 'X': 16, 'b': 2, 'o': 8}[r]
	s := strconv.FormatUint(u, base)
	if r == 'X' {
		return strings.ToUpper(s)
	}
	return s
}

func wjTrunc(s string, p int) string {
	rs := []rune(s)
	if len(rs) > p {
		rs = rs[:p]
	}
	return string(rs)
}

// wjFmt is Rust's Formatter state: width/precision (-1 = none), fill, align (0 = type default),
// and the `0`, `+`, `#` flags. A nil *wjFmt means all defaults. Derived Debug passes it to every
// element, as Rust does (`{:5?}` pads each number; `{:#?}` is the multi-line layout).
type wjFmt struct {
	W, P        int
	Fill, Align rune
	Zero, Plus  bool
	Alt         bool
}

type wjDebugger interface{ wjDbgF(f *wjFmt) string }

func wjIndent4(s string) string { return strings.ReplaceAll(s, "\n", "\n    ") }

func wjDbgList(f *wjFmt, open, close string, items []string) string {
	if f == nil || !f.Alt {
		return open + strings.Join(items, ", ") + close
	}
	if len(items) == 0 {
		return open + close
	}
	var b strings.Builder
	b.WriteString(open + "\n")
	for _, it := range items {
		b.WriteString("    " + wjIndent4(it) + ",\n")
	}
	return b.String() + close
}

// wjDbgTupleS renders `Name(a, b)`; an empty name is a tuple (`(a,)` for one element).
func wjDbgTupleS(f *wjFmt, name string, vals ...any) string {
	if len(vals) == 0 {
		if name == "" {
			return "()"
		}
		return name
	}
	items := make([]string, len(vals))
	for i, v := range vals {
		items[i] = wjDbgO(v, f)
	}
	if name == "" && len(items) == 1 && (f == nil || !f.Alt) {
		return "(" + items[0] + ",)"
	}
	return name + wjDbgList(f, "(", ")", items)
}

// wjDbgStruct renders `Name { a: 1, b: 2 }` (names and values in declaration order).
func wjDbgStruct(f *wjFmt, name string, names []string, vals ...any) string {
	if len(vals) == 0 {
		return name
	}
	items := make([]string, len(vals))
	for i, v := range vals {
		items[i] = names[i] + ": " + wjDbgO(v, f)
	}
	if f != nil && f.Alt {
		return name + " " + wjDbgList(f, "{", "}", items)
	}
	return name + " { " + strings.Join(items, ", ") + " }"
}

// wjPadText is Rust's Formatter::pad: fill/align/width only (default left).
func wjPadText(s string, f *wjFmt, defAlign rune) string {
	if f == nil || f.W < 0 {
		return s
	}
	n := f.W - utf8.RuneCountInString(s)
	if n <= 0 {
		return s
	}
	fill, align := string(f.Fill), f.Align
	if f.Fill == 0 {
		fill = " "
	}
	if align == 0 {
		align = defAlign
	}
	switch align {
	case '>':
		return strings.Repeat(fill, n) + s
	case '^':
		l := n / 2
		return strings.Repeat(fill, l) + s + strings.Repeat(fill, n-l)
	}
	return s + strings.Repeat(fill, n)
}

func wjFmtStr(s string, f *wjFmt) string {
	if f != nil && f.P >= 0 {
		s = wjTrunc(s, f.P)
	}
	return wjPadText(s, f, '<')
}

// wjPadNum is Rust's pad_integral / pad_formatted_parts: sign, prefix, digits; `0` pads between.
func wjPadNum(neg bool, prefix, digits string, f *wjFmt) string {
	sign := ""
	if neg {
		sign = "-"
	} else if f != nil && f.Plus {
		sign = "+"
	}
	s := sign + prefix + digits
	if f == nil || f.W < 0 {
		return s
	}
	if f.Zero {
		if n := f.W - utf8.RuneCountInString(s); n > 0 {
			return sign + prefix + strings.Repeat("0", n) + digits
		}
		return s
	}
	return wjPadText(s, f, '>')
}

// wjFmtInt formats an integer; r is 0 (decimal) or a radix type `x X b o` (two's complement).
func wjFmtInt[T integer](x T, r rune, f *wjFmt) string {
	if r == 0 {
		if isSigned[T]() && int64(x) < 0 {
			return wjPadNum(true, "", strconv.FormatUint(-uint64(int64(x)), 10), f)
		}
		if isSigned[T]() {
			return wjPadNum(false, "", strconv.FormatInt(int64(x), 10), f)
		}
		return wjPadNum(false, "", strconv.FormatUint(uint64(x), 10), f)
	}
	prefix := ""
	if f != nil && f.Alt {
		prefix = map[rune]string{'x': "0x", 'X': "0x", 'b': "0b", 'o': "0o"}[r]
	}
	return wjPadNum(false, prefix, wjRadix(x, r), f)
}

// wjFmtFloat formats a float (debug: `{:?}` digits; exp: `{:e}`); NaN never has a sign.
func wjFmtFloat(x float64, bits int, debug, exp bool, f *wjFmt) string {
	if x != x {
		g := wjFmt{W: -1, P: -1}
		if f != nil {
			g = *f
			g.Plus = false
		}
		return wjPadNum(false, "", "NaN", &g)
	}
	neg := math.Signbit(x)
	a := math.Abs(x)
	var digits string
	switch {
	case exp:
		digits = wjExpFmt(a)
	case f != nil && f.P >= 0:
		digits = wjFixed(a, f.P)
	case debug:
		digits = wjDbgFloat(a, bits)
	case bits == 32:
		digits = wjF32(float32(a))
	default:
		digits = wjF64(a)
	}
	return wjPadNum(neg, "", digits, f)
}

func wjQuote(s string, q rune) string {
	var b strings.Builder
	for _, c := range s {
		switch c {
		case '\n':
			b.WriteString("\\n")
		case '\r':
			b.WriteString("\\r")
		case '\t':
			b.WriteString("\\t")
		case '\\':
			b.WriteString("\\\\")
		case 0:
			b.WriteString("\\0")
		case q:
			b.WriteString("\\" + string(q))
		default:
			if c < 0x20 || c == 0x7f {
				b.WriteString(fmt.Sprintf("\\u{%x}", c))
			} else {
				b.WriteRune(c)
			}
		}
	}
	return b.String()
}

func wjDbg(v any) string { return wjDbgO(v, nil) }

func wjDbgO(v any, f *wjFmt) string {
	if d, ok := v.(wjDebugger); ok {
		return d.wjDbgF(f)
	}
	return wjDbgValue(reflect.ValueOf(v), f)
}

func wjDbgValue(v reflect.Value, f *wjFmt) string {
	if !v.IsValid() {
		return wjPadText("()", f, '<')
	}
	if v.Type() == reflect.TypeOf(Char(0)) {
		return "'" + wjQuote(string(rune(v.Int())), '\'') + "'"
	}
	if v.CanInterface() {
		if d, ok := v.Interface().(wjDebugger); ok {
			return d.wjDbgF(f)
		}
	}
	switch v.Kind() {
	case reflect.Bool:
		return wjPadText(strconv.FormatBool(v.Bool()), f, '<')
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return wjFmtInt(v.Int(), 0, f)
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return wjFmtInt(v.Uint(), 0, f)
	case reflect.Float32:
		return wjFmtFloat(v.Float(), 32, true, false, f)
	case reflect.Float64:
		return wjFmtFloat(v.Float(), 64, true, false, f)
	case reflect.String:
		return "\"" + wjQuote(v.String(), '"') + "\""
	case reflect.Slice, reflect.Array:
		parts := make([]string, v.Len())
		for i := range parts {
			parts[i] = wjDbgValue(v.Index(i), f)
		}
		return wjDbgList(f, "[", "]", parts)
	case reflect.Map:
		keys := v.MapKeys()
		sort.Slice(keys, func(i, j int) bool { return fmt.Sprint(keys[i]) < fmt.Sprint(keys[j]) })
		parts := make([]string, len(keys))
		isSet := v.Type().Elem().Kind() == reflect.Struct && v.Type().Elem().NumField() == 0
		for i, k := range keys {
			if isSet {
				parts[i] = wjDbgValue(k, f)
			} else {
				parts[i] = wjDbgValue(k, f) + ": " + wjDbgValue(v.MapIndex(k), f)
			}
		}
		return wjDbgList(f, "{", "}", parts)
	case reflect.Interface, reflect.Ptr:
		if v.IsNil() {
			return "None"
		}
		return wjDbgValue(v.Elem(), f)
	case reflect.Struct:
		if v.NumField() == 0 {
			return wjPadText("()", f, '<')
		}
	}
	return fmt.Sprint(v.Interface())
}

// ---------------------------------------------------------------------------------------------
// Equality, cloning, misc generics
// ---------------------------------------------------------------------------------------------

func wjEq[T any](a, b T) bool { return reflect.DeepEqual(a, b) }

func wjIs[T any](x any) bool {
	_, ok := x.(T)
	return ok
}

func wjPtr[T any](v T) *T { return &v }

func wjClone[T any](x T) T {
	return wjCopyValue(reflect.ValueOf(&x).Elem()).Interface().(T)
}

func wjCopyValue(v reflect.Value) reflect.Value {
	out := reflect.New(v.Type()).Elem()
	switch v.Kind() {
	case reflect.Slice:
		if v.IsNil() {
			return out
		}
		out.Set(reflect.MakeSlice(v.Type(), v.Len(), v.Len()))
		for i := 0; i < v.Len(); i++ {
			out.Index(i).Set(wjCopyValue(v.Index(i)))
		}
	case reflect.Array:
		for i := 0; i < v.Len(); i++ {
			out.Index(i).Set(wjCopyValue(v.Index(i)))
		}
	case reflect.Map:
		if v.IsNil() {
			return out
		}
		out.Set(reflect.MakeMapWithSize(v.Type(), v.Len()))
		for _, k := range v.MapKeys() {
			out.SetMapIndex(k, wjCopyValue(v.MapIndex(k)))
		}
	case reflect.Struct:
		for i := 0; i < v.NumField(); i++ {
			if out.Field(i).CanSet() {
				out.Field(i).Set(wjCopyValue(v.Field(i)))
			}
		}
	case reflect.Interface:
		if !v.IsNil() {
			out.Set(wjCopyValue(v.Elem()))
		}
	case reflect.Ptr:
		if !v.IsNil() {
			p := reflect.New(v.Type().Elem())
			p.Elem().Set(wjCopyValue(v.Elem()))
			out.Set(p)
		}
	default:
		out.Set(v)
	}
	return out
}

// ---------------------------------------------------------------------------------------------
// Option / Result
// ---------------------------------------------------------------------------------------------

func wjUnwrapOpt[T any](o Option[T]) T {
	if !o.Ok {
		wjPanic("called `Option::unwrap()` on a `None` value")
	}
	return o.V
}

func wjExpectOpt[T any](o Option[T], m string) T {
	if !o.Ok {
		wjPanic(m)
	}
	return o.V
}

func wjUnwrapOr[T any](o Option[T], d T) T {
	if o.Ok {
		return o.V
	}
	return d
}

func wjOptOrElse[T any](o Option[T], f func() T) T {
	if o.Ok {
		return o.V
	}
	return f()
}

func wjOptMap[T, U any](o Option[T], f func(T) U) Option[U] {
	if !o.Ok {
		return Option[U]{}
	}
	return Option[U]{Ok: true, V: f(o.V)}
}

func wjOptAndThen[T, U any](o Option[T], f func(T) Option[U]) Option[U] {
	if !o.Ok {
		return Option[U]{}
	}
	return f(o.V)
}

func wjOptFilter[T any](o Option[T], f func(T) bool) Option[T] {
	if o.Ok && f(o.V) {
		return o
	}
	return Option[T]{}
}

func wjOptIsSomeAnd[T any](o Option[T], f func(T) bool) bool { return o.Ok && f(o.V) }

func wjOkOr[T, E any](o Option[T], e E) Result[T, E] {
	if o.Ok {
		return Result[T, E]{Ok: true, V: o.V}
	}
	return Result[T, E]{E: e}
}

func wjOptMapOr[T, U any](o Option[T], d U, f func(T) U) U {
	if o.Ok {
		return f(o.V)
	}
	return d
}

func wjUnwrapRes[T, E any](r Result[T, E]) T {
	if !r.Ok {
		wjPanic("called `Result::unwrap()` on an `Err` value: " + wjDbg(r.E))
	}
	return r.V
}

func wjExpectRes[T, E any](r Result[T, E], m string) T {
	if !r.Ok {
		wjPanic(m + ": " + wjDbg(r.E))
	}
	return r.V
}

func wjUnwrapErr[T, E any](r Result[T, E]) E {
	if r.Ok {
		wjPanic("called `Result::unwrap_err()` on an `Ok` value")
	}
	return r.E
}

func wjResUnwrapOr[T, E any](r Result[T, E], d T) T {
	if r.Ok {
		return r.V
	}
	return d
}

func wjResOrElse[T, E any](r Result[T, E], f func(E) T) T {
	if r.Ok {
		return r.V
	}
	return f(r.E)
}

func wjResMap[T, E, U any](r Result[T, E], f func(T) U) Result[U, E] {
	if r.Ok {
		return Result[U, E]{Ok: true, V: f(r.V)}
	}
	return Result[U, E]{E: r.E}
}

func wjResMapErr[T, E, F any](r Result[T, E], f func(E) F) Result[T, F] {
	if r.Ok {
		return Result[T, F]{Ok: true, V: r.V}
	}
	return Result[T, F]{E: f(r.E)}
}

func wjResAndThen[T, E, U any](r Result[T, E], f func(T) Result[U, E]) Result[U, E] {
	if r.Ok {
		return f(r.V)
	}
	return Result[U, E]{E: r.E}
}

func wjResOk[T, E any](r Result[T, E]) Option[T] { return Option[T]{Ok: r.Ok, V: r.V} }

func wjResErr[T, E any](r Result[T, E]) Option[E] {
	if r.Ok {
		return Option[E]{}
	}
	return Option[E]{Ok: true, V: r.E}
}

// ---------------------------------------------------------------------------------------------
// Vec / iterators (eager)
// ---------------------------------------------------------------------------------------------

func wjPush[T any](v *[]T, x T)           { *v = append(*v, x) }
func wjPushFront[T any](v *[]T, x T)      { *v = append([]T{x}, *v...) }
func wjClear[T any](v *[]T)               { *v = (*v)[:0] }
func wjExtend[T any](v *[]T, o []T)       { *v = append(*v, o...) }

func wjPop[T any](v *[]T) Option[T] {
	n := len(*v)
	if n == 0 {
		return Option[T]{}
	}
	x := (*v)[n-1]
	*v = (*v)[:n-1]
	return Option[T]{Ok: true, V: x}
}

func wjPopFront[T any](v *[]T) Option[T] {
	if len(*v) == 0 {
		return Option[T]{}
	}
	x := (*v)[0]
	*v = (*v)[1:]
	return Option[T]{Ok: true, V: x}
}

func wjGet[T any, I integer](v []T, i I) Option[T] {
	if i < 0 || uint64(i) >= uint64(len(v)) {
		return Option[T]{}
	}
	return Option[T]{Ok: true, V: v[i]}
}

func wjFirst[T any](v []T) Option[T] { return wjGet(v, 0) }

func wjLast[T any](v []T) Option[T] {
	if len(v) == 0 {
		return Option[T]{}
	}
	return Option[T]{Ok: true, V: v[len(v)-1]}
}

func wjContains[T comparable](v []T, x T) bool { return slices.Contains(v, x) }

func wjContainsEq[T any](v []T, x T) bool {
	for _, e := range v {
		if reflect.DeepEqual(e, x) {
			return true
		}
	}
	return false
}

func wjInsert[T any, I integer](v *[]T, i I, x T) {
	if i < 0 || uint64(i) > uint64(len(*v)) {
		wjPanic("insertion index out of bounds")
	}
	*v = slices.Insert(*v, int(i), x)
}

func wjRemove[T any, I integer](v *[]T, i I) T {
	if i < 0 || uint64(i) >= uint64(len(*v)) {
		wjPanic("removal index out of bounds")
	}
	x := (*v)[i]
	*v = slices.Delete(*v, int(i), int(i)+1)
	return x
}

func wjSwapRemove[T any, I integer](v *[]T, i I) T {
	if i < 0 || uint64(i) >= uint64(len(*v)) {
		wjPanic("swap_remove index out of bounds")
	}
	x := (*v)[i]
	n := len(*v)
	(*v)[i] = (*v)[n-1]
	*v = (*v)[:n-1]
	return x
}

func wjSort[T cmp.Ordered](v []T) { slices.SortStableFunc(v, cmp.Compare[T]) }

func wjReverseInPlace[T any](v []T) { slices.Reverse(v) }

func wjDedup[T any](v *[]T) {
	out := (*v)[:0]
	for i, x := range *v {
		if i == 0 || !reflect.DeepEqual(out[len(out)-1], x) {
			out = append(out, x)
		}
	}
	*v = out
}

func wjTruncate[T any, I integer](v *[]T, n I) {
	if uint64(n) < uint64(len(*v)) {
		*v = (*v)[:n]
	}
}

func wjSwap[T any, I, J integer](v []T, i I, j J) { v[i], v[j] = v[j], v[i] }

func wjDrain[T any](v *[]T) []T {
	out := slices.Clone(*v)
	*v = (*v)[:0]
	return out
}

func wjWindows[T any, I integer](v []T, n I) [][]T {
	var out [][]T
	for i := 0; i+int(n) <= len(v); i++ {
		out = append(out, slices.Clone(v[i:i+int(n)]))
	}
	return out
}

func wjChunks[T any, I integer](v []T, n I) [][]T {
	if n == 0 {
		wjPanic("chunk size must be non-zero")
	}
	var out [][]T
	for i := 0; i < len(v); i += int(n) {
		out = append(out, slices.Clone(v[i:min(i+int(n), len(v))]))
	}
	return out
}

func wjJoin(v []string, sep string) string { return strings.Join(v, sep) }

func wjBsearch[T cmp.Ordered](v []T, x T) Result[uint, uint] {
	i, found := slices.BinarySearch(v, x)
	if found {
		return Result[uint, uint]{Ok: true, V: uint(i)}
	}
	return Result[uint, uint]{E: uint(i)}
}

func wjRetain[T any](v *[]T, f func(T) bool) {
	out := (*v)[:0]
	for _, x := range *v {
		if f(x) {
			out = append(out, x)
		}
	}
	*v = out
}

func wjSplitOff[T any, I integer](v *[]T, n I) []T {
	if uint64(n) > uint64(len(*v)) {
		wjPanic("`at` split index out of bounds")
	}
	tail := slices.Clone((*v)[n:])
	*v = (*v)[:n]
	return tail
}

func wjSplitAt[T any, I integer](v []T, n I) Tuple2[[]T, []T] {
	if uint64(n) > uint64(len(v)) {
		wjPanic("mid > len")
	}
	return Tuple2[[]T, []T]{F0: slices.Clone(v[:n]), F1: slices.Clone(v[n:])}
}

func wjFill[T any](v []T, x T) {
	for i := range v {
		v[i] = x
	}
}

func wjVecSlice[T any, A, B integer](v []T, a A, b B, incl bool) []T {
	s, e := int(a), int(b)
	if incl {
		e++
	}
	if s > e {
		wjPanic(fmt.Sprintf("slice index starts at %d but ends at %d", s, e))
	}
	if e > len(v) {
		wjPanic(fmt.Sprintf("range end index %d out of range for slice of length %d", e, len(v)))
	}
	return v[s:e]
}

func wjRange[T integer](a, b T, incl bool) []T {
	var out []T
	if incl {
		for i := a; i <= b; i++ {
			out = append(out, i)
			if i == maxVal[T]() {
				break
			}
		}
		return out
	}
	for i := a; i < b; i++ {
		out = append(out, i)
	}
	return out
}

func wjRepeat[T any, N integer](x T, n N) []T {
	out := make([]T, int(n))
	for i := range out {
		out[i] = x
	}
	return out
}

func wjRepeatClone[T any, N integer](x T, n N) []T {
	out := make([]T, int(n))
	for i := range out {
		out[i] = wjClone(x)
	}
	return out
}

func wjMap[T, U any](v []T, f func(T) U) []U {
	out := make([]U, len(v))
	for i, x := range v {
		out[i] = f(x)
	}
	return out
}

func wjFilter[T any](v []T, f func(T) bool) []T {
	var out []T
	for _, x := range v {
		if f(x) {
			out = append(out, x)
		}
	}
	return out
}

func wjForEach[T any](v []T, f func(T)) {
	for _, x := range v {
		f(x)
	}
}

func wjFilterMap[T, U any](v []T, f func(T) Option[U]) []U {
	var out []U
	for _, x := range v {
		if r := f(x); r.Ok {
			out = append(out, r.V)
		}
	}
	return out
}

func wjFlatMap[T, U any](v []T, f func(T) []U) []U {
	var out []U
	for _, x := range v {
		out = append(out, f(x)...)
	}
	return out
}

func wjEnumerate[T any](v []T) []Tuple2[uint, T] {
	out := make([]Tuple2[uint, T], len(v))
	for i, x := range v {
		out[i] = Tuple2[uint, T]{F0: uint(i), F1: x}
	}
	return out
}

func wjZip[T, U any](a []T, b []U) []Tuple2[T, U] {
	n := min(len(a), len(b))
	out := make([]Tuple2[T, U], n)
	for i := 0; i < n; i++ {
		out[i] = Tuple2[T, U]{F0: a[i], F1: b[i]}
	}
	return out
}

func wjRev[T any](v []T) []T {
	out := slices.Clone(v)
	slices.Reverse(out)
	return out
}

func wjTake[T any, N integer](v []T, n N) []T { return v[:min(int(n), len(v))] }
func wjSkip[T any, N integer](v []T, n N) []T { return v[min(int(n), len(v)):] }

func wjStepBy[T any, N integer](v []T, n N) []T {
	var out []T
	for i := 0; i < len(v); i += int(n) {
		out = append(out, v[i])
	}
	return out
}

func wjTakeWhile[T any](v []T, f func(T) bool) []T {
	for i, x := range v {
		if !f(x) {
			return v[:i]
		}
	}
	return v
}

func wjSkipWhile[T any](v []T, f func(T) bool) []T {
	for i, x := range v {
		if !f(x) {
			return v[i:]
		}
	}
	return nil
}

func wjChain[T any](a, b []T) []T { return append(slices.Clone(a), b...) }

func wjToSet[T comparable](v []T) map[T]struct{} {
	out := make(map[T]struct{}, len(v))
	for _, x := range v {
		out[x] = struct{}{}
	}
	return out
}

func wjToMap[K comparable, V any](v []Tuple2[K, V]) map[K]V {
	out := make(map[K]V, len(v))
	for _, p := range v {
		out[p.F0] = p.F1
	}
	return out
}

func wjCollectStr[T ~int32 | ~string](v []T) string {
	var b strings.Builder
	for _, x := range v {
		b.WriteString(fmt.Sprint(any(x)))
	}
	return b.String()
}

func wjCollectResult[T, E any](v []Result[T, E]) Result[[]T, E] {
	out := make([]T, 0, len(v))
	for _, r := range v {
		if !r.Ok {
			return Result[[]T, E]{E: r.E}
		}
		out = append(out, r.V)
	}
	return Result[[]T, E]{Ok: true, V: out}
}

func wjCollectOption[T any](v []Option[T]) Option[[]T] {
	out := make([]T, 0, len(v))
	for _, o := range v {
		if !o.Ok {
			return Option[[]T]{}
		}
		out = append(out, o.V)
	}
	return Option[[]T]{Ok: true, V: out}
}

func wjSum[T number](v []T) T {
	var s T
	for _, x := range v {
		s = wjAddAny(s, x)
	}
	return s
}

func wjAddAny[T number](a, b T) T {
	switch any(a).(type) {
	case int:
		return T(wjAdd(int(any(a).(int)), int(any(b).(int))))
	case int8:
		return any(wjAdd(any(a).(int8), any(b).(int8))).(T)
	case int16:
		return any(wjAdd(any(a).(int16), any(b).(int16))).(T)
	case int32:
		return any(wjAdd(any(a).(int32), any(b).(int32))).(T)
	case int64:
		return any(wjAdd(any(a).(int64), any(b).(int64))).(T)
	case uint:
		return any(wjAdd(any(a).(uint), any(b).(uint))).(T)
	case uint8:
		return any(wjAdd(any(a).(uint8), any(b).(uint8))).(T)
	case uint16:
		return any(wjAdd(any(a).(uint16), any(b).(uint16))).(T)
	case uint32:
		return any(wjAdd(any(a).(uint32), any(b).(uint32))).(T)
	case uint64:
		return any(wjAdd(any(a).(uint64), any(b).(uint64))).(T)
	}
	return a + b
}

func wjProduct[T number](v []T) T {
	var p T = 1
	for _, x := range v {
		p *= x
	}
	return p
}

func wjFold[T, A any](v []T, init A, f func(A, T) A) A {
	acc := init
	for _, x := range v {
		acc = f(acc, x)
	}
	return acc
}

func wjAny[T any](v []T, f func(T) bool) bool {
	for _, x := range v {
		if f(x) {
			return true
		}
	}
	return false
}

func wjAll[T any](v []T, f func(T) bool) bool {
	for _, x := range v {
		if !f(x) {
			return false
		}
	}
	return true
}

func wjMinOf[T cmp.Ordered](v []T) Option[T] {
	if len(v) == 0 {
		return Option[T]{}
	}
	return Option[T]{Ok: true, V: slices.Min(v)}
}

func wjMaxOf[T cmp.Ordered](v []T) Option[T] {
	if len(v) == 0 {
		return Option[T]{}
	}
	m := v[0]
	for _, x := range v[1:] {
		if x >= m {
			m = x
		}
	}
	return Option[T]{Ok: true, V: m}
}

func wjFind[T any](v []T, f func(T) bool) Option[T] {
	for _, x := range v {
		if f(x) {
			return Option[T]{Ok: true, V: x}
		}
	}
	return Option[T]{}
}

func wjPosition[T any](v []T, f func(T) bool) Option[uint] {
	for i, x := range v {
		if f(x) {
			return Option[uint]{Ok: true, V: uint(i)}
		}
	}
	return Option[uint]{}
}

// ---------------------------------------------------------------------------------------------
// HashMap / HashSet
// ---------------------------------------------------------------------------------------------

func wjMapInsert[K comparable, V any](m map[K]V, k K, v V) Option[V] {
	old, ok := m[k]
	m[k] = v
	return Option[V]{Ok: ok, V: old}
}

func wjMapGet[K comparable, V any](m map[K]V, k K) Option[V] {
	v, ok := m[k]
	return Option[V]{Ok: ok, V: v}
}

func wjMapIndex[K comparable, V any](m map[K]V, k K) V {
	v, ok := m[k]
	if !ok {
		wjPanic("key not found in map")
	}
	return v
}

func wjMapRemove[K comparable, V any](m map[K]V, k K) Option[V] {
	v, ok := m[k]
	delete(m, k)
	return Option[V]{Ok: ok, V: v}
}

func wjHas[K comparable, V any](m map[K]V, k K) bool {
	_, ok := m[k]
	return ok
}

func wjKeys[K comparable, V any](m map[K]V) []K {
	out := make([]K, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	return out
}

func wjValues[K comparable, V any](m map[K]V) []V {
	out := make([]V, 0, len(m))
	for _, v := range m {
		out = append(out, v)
	}
	return out
}

func wjEntries[K comparable, V any](m map[K]V) []Tuple2[K, V] {
	out := make([]Tuple2[K, V], 0, len(m))
	for k, v := range m {
		out = append(out, Tuple2[K, V]{F0: k, F1: v})
	}
	return out
}

func wjSetInsert[K comparable](m map[K]struct{}, k K) bool {
	_, ok := m[k]
	m[k] = struct{}{}
	return !ok
}

func wjSetRemove[K comparable](m map[K]struct{}, k K) bool {
	_, ok := m[k]
	delete(m, k)
	return ok
}

// ---------------------------------------------------------------------------------------------
// Strings (UTF-8 byte semantics, like Rust)
// ---------------------------------------------------------------------------------------------

func wjStrSlice[A, B integer](s string, a A, b B, incl bool) string {
	st, en := int(a), int(b)
	if incl {
		en++
	}
	if st > en {
		wjPanic(fmt.Sprintf("begin <= end (%d <= %d) when slicing `%s`", st, en, s))
	}
	if en > len(s) {
		wjPanic(fmt.Sprintf("byte index %d is out of bounds of `%s`", en, s))
	}
	if (st < len(s) && !utf8.RuneStart(s[st])) || (en < len(s) && !utf8.RuneStart(s[en])) {
		wjPanic("byte index is not a char boundary")
	}
	return s[st:en]
}

func wjUpper(s string) string      { return strings.ToUpper(s) }
func wjLower(s string) string      { return strings.ToLower(s) }
func wjTrim(s string) string       { return strings.TrimSpace(s) }
func wjTrimStart(s string) string  { return strings.TrimLeftFunc(s, unicode.IsSpace) }
func wjTrimEnd(s string) string    { return strings.TrimRightFunc(s, unicode.IsSpace) }
func wjStrContains(s, t string) bool { return strings.Contains(s, t) }
func wjStrStarts(s, t string) bool   { return strings.HasPrefix(s, t) }
func wjStrEnds(s, t string) bool     { return strings.HasSuffix(s, t) }
func wjStrReplace(s, a, b string) string { return strings.ReplaceAll(s, a, b) }

func wjStrSplit(s, sep string) []string {
	if sep == "" {
		out := []string{""}
		for _, c := range s {
			out = append(out, string(c))
		}
		return append(out, "")
	}
	return strings.Split(s, sep)
}

func wjChars(s string) []Char {
	out := make([]Char, 0, len(s))
	for _, c := range s {
		out = append(out, Char(c))
	}
	return out
}

func wjBytes(s string) []uint8 { return []uint8(s) }

func wjLines(s string) []string {
	if s == "" {
		return nil
	}
	parts := strings.Split(s, "\n")
	if parts[len(parts)-1] == "" {
		parts = parts[:len(parts)-1]
	}
	for i, p := range parts {
		parts[i] = strings.TrimSuffix(p, "\r")
	}
	return parts
}

func wjSplitWs(s string) []string { return strings.Fields(s) }

func wjCharIndices(s string) []Tuple2[uint, Char] {
	var out []Tuple2[uint, Char]
	for i, c := range s {
		out = append(out, Tuple2[uint, Char]{F0: uint(i), F1: Char(c)})
	}
	return out
}

func wjStrFind(s, t string) Option[uint] {
	i := strings.Index(s, t)
	if i < 0 {
		return Option[uint]{}
	}
	return Option[uint]{Ok: true, V: uint(i)}
}

func wjStrRepeat[N integer](s string, n N) string { return strings.Repeat(s, int(n)) }
func wjPushStr(s *string, t string)               { *s += t }
func wjClearStr(s *string)                        { *s = "" }

func wjParseInt[T integer](s string) Result[T, string] {
	fail := func(m string) Result[T, string] { return Result[T, string]{E: m} }
	if s == "" {
		return fail("cannot parse integer from empty string")
	}
	if isSigned[T]() {
		v, err := strconv.ParseInt(s, 10, bitsOf[T]())
		if err != nil {
			return wjParseErr[T](err, s)
		}
		return Result[T, string]{Ok: true, V: T(v)}
	}
	if s[0] == '-' {
		return fail("invalid digit found in string")
	}
	v, err := strconv.ParseUint(s, 10, bitsOf[T]())
	if err != nil {
		return wjParseErr[T](err, s)
	}
	return Result[T, string]{Ok: true, V: T(v)}
}

// wjParseIntRadix mirrors Rust `from_str_radix`: optional sign, digits of the radix, no whitespace or `_`.
func wjParseIntRadix[T integer, R integer](s string, radix R) Result[T, string] {
	fail := func(m string) Result[T, string] { return Result[T, string]{E: m} }
	if s == "" {
		return fail("cannot parse integer from empty string")
	}
	if s[0] == '-' && !isSigned[T]() {
		return fail("invalid digit found in string")
	}
	body := strings.TrimLeft(s, "+-")
	if len(s)-len(body) > 1 || body == "" || strings.ContainsAny(body, "_+-") {
		return fail("invalid digit found in string")
	}
	if isSigned[T]() {
		v, err := strconv.ParseInt(s, int(radix), bitsOf[T]())
		if err != nil {
			return wjParseErr[T](err, s)
		}
		return Result[T, string]{Ok: true, V: T(v)}
	}
	v, err := strconv.ParseUint(strings.TrimPrefix(s, "+"), int(radix), bitsOf[T]())
	if err != nil {
		return wjParseErr[T](err, s)
	}
	return Result[T, string]{Ok: true, V: T(v)}
}

func wjParseErr[T integer](err error, s string) Result[T, string] {
	if ne, ok := err.(*strconv.NumError); ok && ne.Err == strconv.ErrRange {
		if s[0] == '-' {
			return Result[T, string]{E: "number too small to fit in target type"}
		}
		return Result[T, string]{E: "number too large to fit in target type"}
	}
	return Result[T, string]{E: "invalid digit found in string"}
}

func wjParseFloat(s string) Result[float64, string] {
	l := strings.ToLower(s)
	ok := s != "" && !strings.ContainsAny(s, " \t\n_xXpP")
	if ok || l == "inf" || l == "nan" {
		if v, err := strconv.ParseFloat(s, 64); err == nil || (err != nil && math.IsInf(v, 0)) {
			return Result[float64, string]{Ok: true, V: v}
		}
	}
	return Result[float64, string]{E: "invalid float literal"}
}

func wjParseFloat32(s string) Result[float32, string] {
	r := wjParseFloat(s)
	return Result[float32, string]{Ok: r.Ok, V: float32(r.V), E: r.E}
}

func wjParseBool(s string) Result[bool, string] {
	switch s {
	case "true":
		return Result[bool, string]{Ok: true, V: true}
	case "false":
		return Result[bool, string]{Ok: true, V: false}
	}
	return Result[bool, string]{E: "provided string was not `true` or `false`"}
}

func wjCharFromU32[T integer](n T) Option[Char] {
	if n < 0 || uint64(n) > 0x10ffff || (uint64(n) >= 0xd800 && uint64(n) <= 0xdfff) {
		return Option[Char]{}
	}
	return Option[Char]{Ok: true, V: Char(n)}
}

func wjIsAlphabetic(c Char) bool { return unicode.IsLetter(rune(c)) }
func wjIsNumeric(c Char) bool    { return unicode.IsNumber(rune(c)) }
func wjIsWhitespace(c Char) bool { return unicode.IsSpace(rune(c)) }
func wjIsUpper(c Char) bool      { return unicode.IsUpper(rune(c)) }
func wjIsLower(c Char) bool      { return unicode.IsLower(rune(c)) }

func wjAsciiUpper(c Char) Char {
	if c >= 'a' && c <= 'z' {
		return c - 32
	}
	return c
}

func wjAsciiLower(c Char) Char {
	if c >= 'A' && c <= 'Z' {
		return c + 32
	}
	return c
}

func wjToDigit[R integer](c Char, radix R) Option[uint32] {
	var d int
	switch {
	case c >= '0' && c <= '9':
		d = int(c - '0')
	case c >= 'a' && c <= 'z':
		d = int(c-'a') + 10
	case c >= 'A' && c <= 'Z':
		d = int(c-'A') + 10
	default:
		return Option[uint32]{}
	}
	if d >= int(radix) {
		return Option[uint32]{}
	}
	return Option[uint32]{Ok: true, V: uint32(d)}
}
