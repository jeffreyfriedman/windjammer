package main

import (
	"bufio"
	"fmt"
	"os"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	var t int
	fmt.Fscan(r, &t)
	for ; t > 0; t-- {
		var n int64
		fmt.Fscan(r, &n)
		steps, peak := 0, n
		for n != 1 {
			if n%2 == 0 {
				n /= 2
			} else {
				n = 3*n + 1
			}
			steps++
			peak = max(peak, n)
		}
		fmt.Println(steps, peak)
	}
}
