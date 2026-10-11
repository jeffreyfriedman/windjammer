package main

import (
	"bufio"
	"fmt"
	"math/big"
	"os"
	"strings"
)

func valid(s string) bool {
	d := strings.TrimLeft(s[:1], "+-") + s[1:]
	if d == "" {
		return false
	}
	for i := 0; i < len(d); i++ {
		if d[i] < '0' || d[i] > '9' {
			return false
		}
	}
	return true
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	var ok, invalid, overflow, empty int
	lo, hi := big.NewInt(-2147483648), big.NewInt(2147483647)
	for sc.Scan() {
		s := sc.Text()
		switch {
		case s == "":
			empty++
			fmt.Println("error empty")
		case !valid(s):
			invalid++
			fmt.Println("error invalid", s)
		default:
			v, _ := new(big.Int).SetString(strings.TrimPrefix(s, "+"), 10)
			if v.Cmp(lo) < 0 || v.Cmp(hi) > 0 {
				overflow++
				fmt.Println("error overflow", s)
			} else {
				ok++
				fmt.Println("ok", v)
			}
		}
	}
	fmt.Printf("summary ok=%d invalid=%d overflow=%d empty=%d\n", ok, invalid, overflow, empty)
}
