package main

import (
	"bufio"
	"fmt"
	"os"
	"slices"
	"strings"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Buffer(make([]byte, 1<<20), 1<<26)
	sc.Split(bufio.ScanWords)
	seen := map[string]bool{}
	groups := map[string][]string{}
	for sc.Scan() {
		w := strings.ToLower(sc.Text())
		if seen[w] {
			continue
		}
		seen[w] = true
		b := []byte(w)
		slices.Sort(b)
		groups[string(b)] = append(groups[string(b)], w)
	}
	var lines [][]string
	for _, g := range groups {
		if len(g) >= 2 {
			slices.Sort(g)
			lines = append(lines, g)
		}
	}
	if len(lines) == 0 {
		fmt.Println("none")
		return
	}
	slices.SortFunc(lines, func(a, b []string) int { return strings.Compare(a[0], b[0]) })
	for _, g := range lines {
		fmt.Println(strings.Join(g, " "))
	}
}
