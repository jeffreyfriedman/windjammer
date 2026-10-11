package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func plainInt(s string, lo, hi int) (int, bool) {
	if s == "" {
		return 0, false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return 0, false
		}
	}
	v, err := strconv.Atoi(s)
	return v, err == nil && v >= lo && v <= hi
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	vals := map[string]string{}
	var unknown, problems []string
	known := map[string]bool{"host": true, "port": true, "debug": true, "workers": true}
	seenUnknown := map[string]bool{}
	for n := 1; sc.Scan(); n++ {
		line := sc.Text()
		if line == "" {
			continue
		}
		k, v, ok := strings.Cut(line, "=")
		if !ok {
			problems = append(problems, fmt.Sprintf("error: malformed line %d", n))
			continue
		}
		vals[k] = v
		if !known[k] && !seenUnknown[k] {
			seenUnknown[k] = true
			unknown = append(unknown, k)
		}
	}
	host, hasHost := vals["host"]
	switch {
	case !hasHost:
		problems = append(problems, "error: host: missing")
	case host == "":
		problems = append(problems, "error: host: empty")
	}
	portS, hasPort := vals["port"]
	port, ok := plainInt(portS, 1, 65535)
	switch {
	case !hasPort:
		problems = append(problems, "error: port: missing")
	case !ok:
		problems = append(problems, "error: port: invalid value "+portS)
	}
	debug := "false"
	if d, has := vals["debug"]; has {
		if d == "true" || d == "false" {
			debug = d
		} else {
			problems = append(problems, "error: debug: invalid value "+d)
		}
	}
	workers := 4
	if w, has := vals["workers"]; has {
		if v, ok := plainInt(w, 1, 64); ok {
			workers = v
		} else {
			problems = append(problems, "error: workers: invalid value "+w)
		}
	}
	for _, k := range unknown {
		problems = append(problems, "error: unknown key "+k)
	}
	if len(problems) > 0 {
		fmt.Println(strings.Join(problems, "\n"))
		return
	}
	fmt.Printf("ok host=%s port=%d debug=%s workers=%d\n", host, port, debug, workers)
}
