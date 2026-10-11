package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	var t int
	fmt.Fscan(r, &t)
	for ; t > 0; t-- {
		var n uint64
		fmt.Fscan(r, &n)
		s := strconv.FormatUint(n, 10)
		sum := 0
		rev := make([]byte, len(s))
		for i := range s {
			sum += int(s[i] - '0')
			rev[len(s)-1-i] = s[i]
		}
		root := sum
		for root >= 10 {
			x := 0
			for ; root > 0; root /= 10 {
				x += root % 10
			}
			root = x
		}
		reversed, _ := strconv.ParseUint(string(rev), 10, 64)
		pal := "no"
		if string(rev) == s {
			pal = "yes"
		}
		fmt.Println(sum, root, reversed, pal)
	}
}
