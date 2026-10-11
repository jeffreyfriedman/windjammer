package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func eval(line string) string {
	var st []int64
	for _, tok := range strings.Split(line, " ") {
		if v, err := strconv.ParseInt(tok, 10, 64); err == nil {
			st = append(st, v)
			continue
		}
		if tok != "+" && tok != "-" && tok != "*" && tok != "/" {
			return "error: token"
		}
		if len(st) < 2 {
			return "error: underflow"
		}
		a, b := st[len(st)-2], st[len(st)-1]
		st = st[:len(st)-2]
		var r int64
		switch tok {
		case "+":
			r = a + b
		case "-":
			r = a - b
		case "*":
			r = a * b
		case "/":
			if b == 0 {
				return "error: divzero"
			}
			r = a / b
		}
		st = append(st, r)
	}
	if len(st) != 1 {
		return "error: leftover"
	}
	return strconv.FormatInt(st[0], 10)
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" {
			continue
		}
		fmt.Println(eval(line))
	}
}
