package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	data := map[string]map[string]string{}
	section := ""
	inQueries := false
	for sc.Scan() {
		line := sc.Text()
		if !inQueries {
			if line == "---" {
				inQueries = true
				continue
			}
			t := strings.TrimSpace(line)
			switch {
			case t == "" || t[0] == ';' || t[0] == '#':
			case t[0] == '[' && t[len(t)-1] == ']':
				section = t[1 : len(t)-1]
			default:
				k, v, ok := strings.Cut(t, "=")
				if !ok {
					continue
				}
				if data[section] == nil {
					data[section] = map[string]string{}
				}
				data[section][strings.TrimSpace(k)] = strings.TrimSpace(v)
			}
			continue
		}
		sec, key, ok := strings.Cut(line, ".")
		if !ok {
			sec, key = "", line
		}
		if v, found := data[sec][key]; found {
			fmt.Println(v)
		} else {
			fmt.Println("<none>")
		}
	}
}
