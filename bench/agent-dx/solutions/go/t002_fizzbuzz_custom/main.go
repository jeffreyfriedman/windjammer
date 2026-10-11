package main

import (
	"bufio"
	"fmt"
	"os"
)

func main() {
	var n, a, b int
	fmt.Fscan(bufio.NewReader(os.Stdin), &n, &a, &b)
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	for i := 1; i <= n; i++ {
		da, db := i%a == 0, i%b == 0
		switch {
		case da && db:
			fmt.Fprintln(w, "FizzBuzz")
		case da:
			fmt.Fprintln(w, "Fizz")
		case db:
			fmt.Fprintln(w, "Buzz")
		default:
			fmt.Fprintln(w, i)
		}
	}
}
