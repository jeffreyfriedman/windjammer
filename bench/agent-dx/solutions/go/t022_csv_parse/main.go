package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func parse(line string) []string {
	var fields []string
	var cur strings.Builder
	inQuotes := false
	for i := 0; i < len(line); i++ {
		c := line[i]
		switch {
		case inQuotes && c == '"' && i+1 < len(line) && line[i+1] == '"':
			cur.WriteByte('"')
			i++
		case c == '"':
			inQuotes = !inQuotes
		case c == ',' && !inQuotes:
			fields = append(fields, cur.String())
			cur.Reset()
		default:
			cur.WriteByte(c)
		}
	}
	return append(fields, cur.String())
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		f := parse(sc.Text())
		for i := range f {
			f[i] = "[" + f[i] + "]"
		}
		fmt.Printf("%d: %s\n", len(f), strings.Join(f, "|"))
	}
}
