package main

import (
	"bufio"
	"fmt"
	"math"
	"os"
	"strconv"
	"strings"
)

func inRange(v int64) bool { return v >= math.MinInt32 && v <= math.MaxInt32 }

func compute(op string, a, b int64) string {
	var r int64
	switch op {
	case "add":
		r = a + b
	case "sub":
		r = a - b
	case "mul":
		r = a * b
	case "div", "rem":
		if b == 0 {
			return "divzero"
		}
		if op == "div" {
			r = a / b
		} else {
			r = a % b
		}
	case "pow":
		if b < 0 {
			return "invalid"
		}
		r = 1
		switch {
		case a == 0 && b > 0:
			r = 0
		case a == 1:
		case a == -1 && b%2 == 1:
			r = -1
		case a != -1 && b > 0:
			for i := int64(0); i < b; i++ {
				r *= a
				if !inRange(r) {
					return "overflow"
				}
			}
		}
	}
	if !inRange(r) {
		return "overflow"
	}
	return strconv.FormatInt(r, 10)
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) != 3 {
			continue
		}
		a, _ := strconv.ParseInt(f[1], 10, 64)
		b, _ := strconv.ParseInt(f[2], 10, 64)
		fmt.Println(compute(f[0], a, b))
	}
}
