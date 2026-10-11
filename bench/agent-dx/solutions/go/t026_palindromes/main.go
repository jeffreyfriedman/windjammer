package main

import (
	"bufio"
	"fmt"
	"os"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		s := sc.Text()
		best, bestStart, count := 0, 0, 0
		for c := 0; c < 2*len(s); c++ {
			l, r := c/2, c/2+c%2
			for l >= 0 && r < len(s) && s[l] == s[r] {
				count++
				if r-l+1 > best || (r-l+1 == best && l < bestStart) {
					best, bestStart = r-l+1, l
				}
				l--
				r++
			}
		}
		if best == 0 {
			fmt.Println("0 - 0")
		} else {
			fmt.Println(best, s[bestStart:bestStart+best], count)
		}
	}
}
