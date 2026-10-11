package main

import (
	"bufio"
	"fmt"
	"os"
	"slices"
	"strconv"
	"strings"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	var l []int64
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		var x int64
		if len(f) > 1 {
			x, _ = strconv.ParseInt(f[1], 10, 64)
		}
		switch f[0] {
		case "push_front":
			l = append([]int64{x}, l...)
		case "push_back":
			l = append(l, x)
		case "pop_front":
			if len(l) == 0 {
				fmt.Println("empty")
			} else {
				fmt.Println(l[0])
				l = l[1:]
			}
		case "pop_back":
			if len(l) == 0 {
				fmt.Println("empty")
			} else {
				fmt.Println(l[len(l)-1])
				l = l[:len(l)-1]
			}
		case "remove":
			if i := slices.Index(l, x); i >= 0 {
				l = slices.Delete(l, i, i+1)
				fmt.Println("removed")
			} else {
				fmt.Println("missing")
			}
		case "reverse":
			slices.Reverse(l)
		case "print":
			if len(l) == 0 {
				fmt.Println("<empty>")
				continue
			}
			s := make([]string, len(l))
			for i, v := range l {
				s[i] = strconv.FormatInt(v, 10)
			}
			fmt.Println(strings.Join(s, " "))
		case "nth":
			if x < 0 || x >= int64(len(l)) {
				fmt.Println("none")
			} else {
				fmt.Println(l[x])
			}
		}
	}
}
