package main

import (
	"bufio"
	"fmt"
	"os"
)

func gcd(a, b int64) int64 {
	for b != 0 {
		a, b = b, a%b
	}
	return a
}

func main() {
	r := bufio.NewReader(os.Stdin)
	var t int
	fmt.Fscan(r, &t)
	for i := 0; i < t; i++ {
		var a, b int64
		fmt.Fscan(r, &a, &b)
		g := gcd(a, b)
		fmt.Println(g, a/g*b)
	}
}
