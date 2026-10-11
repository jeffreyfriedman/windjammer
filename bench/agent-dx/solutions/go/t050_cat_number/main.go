package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"
)

func main() {
	number := false
	var files []string
	for _, a := range os.Args[1:] {
		if a == "-n" {
			number = true
		} else {
			files = append(files, a)
		}
	}
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	lineNo := 0
	emit := func(data []byte) {
		if !number {
			w.Write(data)
			return
		}
		text := string(data)
		for text != "" {
			line, rest, _ := strings.Cut(text, "\n")
			lineNo++
			fmt.Fprintf(w, "%6d\t%s\n", lineNo, line)
			text = rest
		}
	}
	status := 0
	if len(files) == 0 {
		data, _ := io.ReadAll(os.Stdin)
		emit(data)
	}
	for _, name := range files {
		data, err := os.ReadFile(name)
		if err != nil {
			fmt.Fprintf(w, "cat: %s: No such file\n", name)
			status = 1
			continue
		}
		emit(data)
	}
	w.Flush()
	os.Exit(status)
}
