package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

func fail(code int, format string, a ...any) {
	fmt.Printf(format+"\n", a...)
	os.Exit(code)
}

func main() {
	if len(os.Args) != 4 {
		fail(2, "usage: csvcol FILE COLUMN OP")
	}
	file, column, op := os.Args[1], os.Args[2], os.Args[3]
	data, err := os.ReadFile(file)
	if err != nil {
		fail(1, "error: cannot read %s", file)
	}
	lines := strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
	col := -1
	for i, h := range strings.Split(lines[0], ",") {
		if h == column {
			col = i
			break
		}
	}
	if col < 0 {
		fail(2, "error: no such column %s", column)
	}
	switch op {
	case "sum", "min", "max", "count", "mean":
	default:
		fail(2, "error: unknown op %s", op)
	}
	var vals []int64
	n := 0
	for i, line := range lines[1:] {
		if line == "" {
			continue
		}
		n++
		if op == "count" {
			continue
		}
		fields := strings.Split(line, ",")
		v := ""
		if col < len(fields) {
			v = fields[col]
		}
		x, ok := parseNonNeg(v)
		if !ok {
			fail(3, "error: bad value %s on line %d", v, i+2)
		}
		vals = append(vals, x)
	}
	if op == "count" {
		fmt.Println(n)
		return
	}
	if len(vals) == 0 {
		if op == "sum" {
			fmt.Println(0)
		} else {
			fmt.Println("n/a")
		}
		return
	}
	lo, hi, sum := vals[0], vals[0], int64(0)
	for _, v := range vals {
		lo, hi, sum = min(lo, v), max(hi, v), sum+v
	}
	switch op {
	case "sum":
		fmt.Println(sum)
	case "min":
		fmt.Println(lo)
	case "max":
		fmt.Println(hi)
	case "mean":
		fmt.Println(sum / int64(len(vals)))
	}
}

func parseNonNeg(s string) (int64, bool) {
	if s == "" {
		return 0, false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return 0, false
		}
	}
	v, err := strconv.ParseInt(s, 10, 64)
	return v, err == nil
}
