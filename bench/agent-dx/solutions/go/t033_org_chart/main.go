package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type emp struct {
	manager string
	salary  string
}

func main() {
	r := bufio.NewReader(os.Stdin)
	var n int
	fmt.Fscan(r, &n)
	emps := map[string]emp{}
	for i := 0; i < n; i++ {
		var name, mgr, sal string
		fmt.Fscan(r, &name, &mgr, &sal)
		emps[name] = emp{mgr, sal}
	}
	sc := bufio.NewScanner(r)
	for sc.Scan() {
		q := strings.TrimSpace(sc.Text())
		if q == "" {
			continue
		}
		fmt.Println(skip(emps, q))
	}
}

func skip(emps map[string]emp, q string) string {
	e, ok := emps[q]
	if !ok {
		return "none: unknown " + q
	}
	if e.manager == "-" {
		return "none: no manager " + q
	}
	m, ok := emps[e.manager]
	if !ok {
		return "none: unknown " + e.manager
	}
	if m.manager == "-" {
		return "none: no skip-level " + e.manager
	}
	g, ok := emps[m.manager]
	if !ok {
		return "none: unknown " + m.manager
	}
	v, _ := strconv.ParseInt(g.salary, 10, 64)
	return strconv.FormatInt(v, 10)
}
