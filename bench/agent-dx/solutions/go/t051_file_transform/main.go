package main

import (
	"fmt"
	"os"
	"slices"
	"strings"
)

func transform(lines []string, mode string) ([]string, bool) {
	out := slices.Clone(lines)
	switch mode {
	case "upper":
		for i := range out {
			out[i] = strings.ToUpper(out[i])
		}
	case "reverse":
		slices.Reverse(out)
	case "sort":
		slices.Sort(out)
	case "dedupe":
		seen := map[string]bool{}
		out = out[:0]
		for _, l := range lines {
			if !seen[l] {
				seen[l] = true
				out = append(out, l)
			}
		}
	case "trim":
		for i := range out {
			out[i] = strings.Trim(out[i], " ")
		}
	case "number":
		for i := range out {
			out[i] = fmt.Sprintf("%d:%s", i+1, out[i])
		}
	default:
		return nil, false
	}
	return out, true
}

func main() {
	if len(os.Args) != 4 {
		fmt.Println("usage: transform INPUT OUTPUT MODE")
		os.Exit(2)
	}
	input, output, mode := os.Args[1], os.Args[2], os.Args[3]
	data, err := os.ReadFile(input)
	if err != nil {
		fmt.Println("error: cannot read", input)
		os.Exit(1)
	}
	var lines []string
	if len(data) > 0 {
		lines = strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
	}
	out, ok := transform(lines, mode)
	if !ok {
		fmt.Println("error: unknown mode", mode)
		os.Exit(2)
	}
	var sb strings.Builder
	for _, l := range out {
		sb.WriteString(l + "\n")
	}
	if err := os.WriteFile(output, []byte(sb.String()), 0o644); err != nil {
		fmt.Println("error: cannot write", output)
		os.Exit(1)
	}
	back, _ := os.ReadFile(output)
	fmt.Printf("wrote %d lines to %s\n", len(out), output)
	fmt.Print(string(back))
}
