package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func main() {
	var ignoreCase, number, countOnly, invert bool
	var rest []string
	for _, a := range os.Args[1:] {
		switch a {
		case "-i":
			ignoreCase = true
		case "-n":
			number = true
		case "-c":
			countOnly = true
		case "-v":
			invert = true
		default:
			rest = append(rest, a)
		}
	}
	if len(rest) < 2 {
		os.Exit(2)
	}
	pattern, files := rest[0], rest[1:]
	if ignoreCase {
		pattern = strings.ToLower(pattern)
	}
	status, total := 0, 0
	for _, name := range files {
		f, err := os.Open(name)
		if err != nil {
			fmt.Printf("grep: %s: No such file\n", name)
			status = 2
			continue
		}
		prefix := ""
		if len(files) > 1 {
			prefix = name + ":"
		}
		matched := 0
		sc := bufio.NewScanner(f)
		sc.Buffer(make([]byte, 1<<20), 1<<26)
		for n := 1; sc.Scan(); n++ {
			line := sc.Text()
			hay := line
			if ignoreCase {
				hay = strings.ToLower(line)
			}
			if strings.Contains(hay, pattern) == invert {
				continue
			}
			matched++
			if !countOnly {
				if number {
					fmt.Printf("%s%d:%s\n", prefix, n, line)
				} else {
					fmt.Println(prefix + line)
				}
			}
		}
		f.Close()
		total += matched
		if countOnly {
			fmt.Printf("%s%d\n", prefix, matched)
		}
	}
	if status == 0 && total == 0 {
		status = 1
	}
	os.Exit(status)
}
