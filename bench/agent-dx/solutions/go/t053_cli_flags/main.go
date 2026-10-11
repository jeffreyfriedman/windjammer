package main

import (
	"fmt"
	"os"
	"strings"
)

func plainUint(s string) bool {
	if s == "" {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return false
		}
	}
	return true
}

func fail(msg string) {
	fmt.Println(msg)
	os.Exit(2)
}

func main() {
	args := os.Args[1:]
	name, count, verbosity, dry := "anonymous", "1", 0, false
	var positional []string
	setCount := func(v string) {
		if !plainUint(v) {
			fail("error: invalid count " + v)
		}
		count = strings.TrimLeft(v, "0")
		if count == "" {
			count = "0"
		}
	}
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch {
		case a == "--":
			positional = append(positional, args[i+1:]...)
			i = len(args)
		case strings.HasPrefix(a, "--name="):
			name = strings.TrimPrefix(a, "--name=")
		case a == "--name":
			if i+1 >= len(args) {
				fail("error: missing value for --name")
			}
			i++
			name = args[i]
		case strings.HasPrefix(a, "--count="):
			setCount(strings.TrimPrefix(a, "--count="))
		case a == "--count":
			if i+1 >= len(args) {
				fail("error: missing value for --count")
			}
			i++
			setCount(args[i])
		case a == "--dry-run":
			dry = true
		case len(a) > 1 && a[0] == '-' && strings.Trim(a[1:], "v") == "":
			verbosity += len(a) - 1
		case len(a) > 1 && a[0] == '-':
			fail("error: unknown option " + a)
		default:
			positional = append(positional, a)
		}
	}
	fmt.Println("name:", name)
	fmt.Println("count:", count)
	fmt.Println("verbosity:", verbosity)
	fmt.Println("dry-run:", dry)
	fmt.Println("positional: " + strings.Join(positional, " "))
}
