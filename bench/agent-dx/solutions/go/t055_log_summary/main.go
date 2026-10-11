package main

import (
	"bufio"
	"fmt"
	"os"
	"regexp"
	"sort"
)

var lineRe = regexp.MustCompile(`^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}) (DEBUG|INFO|WARN|ERROR) (\S+)(?: (.*))?$`)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Buffer(make([]byte, 1<<20), 1<<26)
	levels := map[string]int{}
	comps := map[string]int{}
	var errs []string
	first, last := "-", "-"
	malformed := 0
	for sc.Scan() {
		m := lineRe.FindStringSubmatch(sc.Text())
		if m == nil {
			malformed++
			continue
		}
		ts, level, comp, msg := m[1], m[2], m[3], m[4]
		levels[level]++
		comps[comp]++
		if first == "-" || ts < first {
			first = ts
		}
		if last == "-" || ts > last {
			last = ts
		}
		if level == "ERROR" {
			errs = append(errs, fmt.Sprintf("  [%s] %s", comp, msg))
		}
	}
	fmt.Printf("levels: DEBUG=%d INFO=%d WARN=%d ERROR=%d\n",
		levels["DEBUG"], levels["INFO"], levels["WARN"], levels["ERROR"])
	names := make([]string, 0, len(comps))
	for c := range comps {
		names = append(names, c)
	}
	sort.Slice(names, func(i, j int) bool {
		if comps[names[i]] != comps[names[j]] {
			return comps[names[i]] > comps[names[j]]
		}
		return names[i] < names[j]
	})
	fmt.Println("top components:")
	for i := 0; i < len(names) && i < 3; i++ {
		fmt.Printf("  %s %d\n", names[i], comps[names[i]])
	}
	fmt.Println("first:", first)
	fmt.Println("last:", last)
	fmt.Println("errors:")
	for _, e := range errs {
		fmt.Println(e)
	}
	fmt.Println("malformed:", malformed)
}
