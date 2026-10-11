package main

import (
	"fmt"
	"io"
	"os"
)

func count(data []byte) (lines, words, size int) {
	inWord := false
	for _, b := range data {
		if b == '\n' {
			lines++
		}
		space := b == ' ' || b == '\t' || b == '\n' || b == '\r'
		if !space && !inWord {
			words++
		}
		inWord = !space
	}
	return lines, words, len(data)
}

func main() {
	args := os.Args[1:]
	if len(args) == 0 {
		data, _ := io.ReadAll(os.Stdin)
		l, w, b := count(data)
		fmt.Println(l, w, b)
		return
	}
	var tl, tw, tb int
	status := 0
	for _, name := range args {
		data, err := os.ReadFile(name)
		if err != nil {
			fmt.Printf("wc: %s: No such file\n", name)
			status = 1
			continue
		}
		l, w, b := count(data)
		tl, tw, tb = tl+l, tw+w, tb+b
		fmt.Println(l, w, b, name)
	}
	if len(args) > 1 {
		fmt.Println(tl, tw, tb, "total")
	}
	os.Exit(status)
}
