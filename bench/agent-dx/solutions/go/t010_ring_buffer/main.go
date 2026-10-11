package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Scan()
	capacity, _ := strconv.Atoi(strings.TrimSpace(sc.Text()))
	var q []string
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		switch f[0] {
		case "push":
			if len(q) >= capacity {
				fmt.Println("full")
			} else {
				q = append(q, f[1])
				fmt.Println("ok")
			}
		case "pop":
			if len(q) == 0 {
				fmt.Println("empty")
			} else {
				fmt.Println(q[0])
				q = q[1:]
			}
		case "peek":
			if len(q) == 0 {
				fmt.Println("empty")
			} else {
				fmt.Println(q[0])
			}
		case "size":
			fmt.Println(len(q))
		case "dump":
			if len(q) == 0 {
				fmt.Println("<empty>")
			} else {
				fmt.Println(strings.Join(q, " "))
			}
		}
	}
}
