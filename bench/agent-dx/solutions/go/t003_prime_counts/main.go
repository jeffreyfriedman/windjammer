package main

import (
	"bufio"
	"fmt"
	"os"
)

const limit = 2000000

func main() {
	composite := make([]bool, limit+1)
	count := make([]int, limit+1)
	largest := make([]int, limit+1)
	for i := 2; i <= limit; i++ {
		count[i], largest[i] = count[i-1], largest[i-1]
		if composite[i] {
			continue
		}
		count[i]++
		largest[i] = i
		for j := i * i; j <= limit; j += i {
			composite[j] = true
		}
	}
	r := bufio.NewReader(os.Stdin)
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	var t int
	fmt.Fscan(r, &t)
	for ; t > 0; t-- {
		var n int
		fmt.Fscan(r, &n)
		if n < 2 {
			fmt.Fprintln(w, "0 none")
		} else {
			fmt.Fprintln(w, count[n], largest[n])
		}
	}
}
