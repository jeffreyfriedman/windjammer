package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

var parent, size []int

func find(x int) int {
	for parent[x] != x {
		parent[x] = parent[parent[x]]
		x = parent[x]
	}
	return x
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Scan()
	n, _ := strconv.Atoi(strings.TrimSpace(sc.Text()))
	parent, size = make([]int, n), make([]int, n)
	for i := range parent {
		parent[i], size[i] = i, 1
	}
	components := n
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) < 2 {
			continue
		}
		a, _ := strconv.Atoi(f[1])
		ra := find(a)
		switch f[0] {
		case "union", "find":
			b, _ := strconv.Atoi(f[2])
			rb := find(b)
			if f[0] == "find" {
				fmt.Println(map[bool]string{true: "yes", false: "no"}[ra == rb])
			} else if ra == rb {
				fmt.Println("same")
			} else {
				parent[rb] = ra
				size[ra] += size[rb]
				components--
				fmt.Println("merged")
			}
		case "size":
			fmt.Println(size[ra])
		}
	}
	fmt.Println("components", components)
}
