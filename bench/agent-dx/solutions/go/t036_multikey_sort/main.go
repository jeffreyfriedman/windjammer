package main

import (
	"bufio"
	"cmp"
	"fmt"
	"os"
	"slices"
	"strconv"
	"strings"
)

type key struct {
	field string
	desc  bool
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Scan()
	var keys []key
	for _, part := range strings.Split(strings.TrimSpace(sc.Text()), ",") {
		f, d, _ := strings.Cut(part, ":")
		keys = append(keys, key{f, d == "desc"})
	}
	var recs [][]string
	var lines []string
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) != 3 {
			continue
		}
		recs = append(recs, f)
		lines = append(lines, sc.Text())
	}
	idx := make([]int, len(recs))
	for i := range idx {
		idx[i] = i
	}
	col := map[string]int{"name": 0, "age": 1, "city": 2}
	slices.SortStableFunc(idx, func(a, b int) int {
		for _, k := range keys {
			c := col[k.field]
			var r int
			if k.field == "age" {
				x, _ := strconv.Atoi(recs[a][c])
				y, _ := strconv.Atoi(recs[b][c])
				r = cmp.Compare(x, y)
			} else {
				r = strings.Compare(recs[a][c], recs[b][c])
			}
			if k.desc {
				r = -r
			}
			if r != 0 {
				return r
			}
		}
		return 0
	})
	for _, i := range idx {
		fmt.Println(lines[i])
	}
}
