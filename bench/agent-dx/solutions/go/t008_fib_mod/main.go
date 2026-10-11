package main

import (
	"bufio"
	"fmt"
	"math/bits"
	"os"
)

func mulmod(a, b, m uint64) uint64 {
	hi, lo := bits.Mul64(a, b)
	_, rem := bits.Div64(hi%m, lo, m)
	return rem
}

// fib returns (F(n), F(n+1)) mod m by fast doubling.
func fib(n, m uint64) (uint64, uint64) {
	if n == 0 {
		return 0, 1 % m
	}
	a, b := fib(n/2, m)
	c := mulmod(a, (2*b+m-a)%m, m)
	d := (mulmod(a, a, m) + mulmod(b, b, m)) % m
	if n%2 == 0 {
		return c, d
	}
	return d, (c + d) % m
}

func main() {
	r := bufio.NewReader(os.Stdin)
	var t int
	fmt.Fscan(r, &t)
	for ; t > 0; t-- {
		var n, m uint64
		fmt.Fscan(r, &n, &m)
		f, _ := fib(n, m)
		fmt.Println(f)
	}
}
