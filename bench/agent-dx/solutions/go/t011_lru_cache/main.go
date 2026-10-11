package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type entry struct{ k, v string }

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Scan()
	capacity, _ := strconv.Atoi(strings.TrimSpace(sc.Text()))
	var items []entry // most recently used first
	find := func(k string) int {
		for i, e := range items {
			if e.k == k {
				return i
			}
		}
		return -1
	}
	toFront := func(i int, e entry) {
		items = append(items[:i], items[i+1:]...)
		items = append([]entry{e}, items...)
	}
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		switch f[0] {
		case "put":
			if i := find(f[1]); i >= 0 {
				toFront(i, entry{f[1], f[2]})
			} else {
				items = append([]entry{{f[1], f[2]}}, items...)
				if len(items) > capacity {
					fmt.Println("evict", items[len(items)-1].k)
					items = items[:len(items)-1]
				}
			}
		case "get":
			if i := find(f[1]); i >= 0 {
				e := items[i]
				fmt.Println(e.v)
				toFront(i, e)
			} else {
				fmt.Println(-1)
			}
		case "keys":
			if len(items) == 0 {
				fmt.Println("<empty>")
				continue
			}
			keys := make([]string, len(items))
			for i, e := range items {
				keys[i] = e.k
			}
			fmt.Println(strings.Join(keys, " "))
		}
	}
}
