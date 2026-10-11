package main

import (
	"bufio"
	"fmt"
	"os"
)

func check(line string) string {
	pairs := map[byte]byte{')': '(', ']': '[', '}': '{'}
	var stack []int
	for i := 0; i < len(line); i++ {
		c := line[i]
		switch c {
		case '(', '[', '{':
			stack = append(stack, i)
		case ')', ']', '}':
			if len(stack) == 0 || line[stack[len(stack)-1]] != pairs[c] {
				return fmt.Sprint("error ", i+1)
			}
			stack = stack[:len(stack)-1]
		}
	}
	if len(stack) > 0 {
		return fmt.Sprint("error ", stack[len(stack)-1]+1)
	}
	return "ok"
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		fmt.Println(check(sc.Text()))
	}
}
